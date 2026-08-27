use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use crate::discovery::types::{DiscoveredDevice, NetworkInterfaceInfo, DiscoveryProgress};
use crate::discovery::network_interfaces::NetworkInterfaceManager;
use crate::discovery::providers::sadp::SadpProvider;
use crate::discovery::providers::onvif::OnvifProvider;
use crate::discovery::providers::ssdp::SsdpProvider;
use crate::discovery::providers::arp::ArpProvider;
use crate::discovery::providers::tcp::TcpPortProvider;
use crate::discovery::providers::http::HttpFingerprintProvider;
use crate::discovery::deduplicator::Deduplicator;
use crate::discovery::diagnostic::DiagnosticEngine;
use crate::discovery::classifier::{classify_device, ClassificationContext};

pub struct DiscoveryEngine;

impl DiscoveryEngine {
    pub async fn run_discovery(
        interface_name: Option<String>,
        progress_callback: impl Fn(DiscoveryProgress) + Send + Sync + 'static,
    ) -> Vec<DiscoveredDevice> {
        let interfaces = NetworkInterfaceManager::get_interfaces();
        let selected_iface = if let Some(ref name) = interface_name {
            interfaces.iter().find(|i| &i.id == name || &i.name == name).cloned()
        } else {
            interfaces.iter().find(|i| i.is_default).or_else(|| interfaces.first()).cloned()
        }.unwrap_or_else(|| NetworkInterfaceInfo {
            id: "eth0".to_string(),
            name: "Ethernet".to_string(),
            ip: "172.20.120.30".to_string(),
            netmask: "255.255.255.0".to_string(),
            broadcast: "172.20.120.255".to_string(),
            gateway: Some("172.20.120.1".to_string()),
            mac: None,
            is_up: true,
            is_default: true,
        });

        let mut deduplicator = Deduplicator::new();
        let broadcast_targets = vec![selected_iface.broadcast.clone(), "255.255.255.255".to_string()];
        let default_gw_ip = selected_iface.gateway.clone().unwrap_or_default();

        // Phase 1: ARP Inspection (10%)
        progress_callback(DiscoveryProgress {
            percentage: 10,
            phase: "Consultando tabela ARP e vizinhos de rede...".to_string(),
            devices_found: 0,
            active_protocols: vec!["ARP".to_string()],
            completed_protocols: Vec::new(),
            is_running: true,
        });

        let arp_table = ArpProvider::get_arp_table();

        // Phase 2: SADP + ONVIF + SSDP/UPnP UDP Broadcasts (30%)
        progress_callback(DiscoveryProgress {
            percentage: 30,
            phase: "Enviando sondas ONVIF WS-Discovery, Hikvision SADP e SSDP/UPnP...".to_string(),
            devices_found: 0,
            active_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string(), "SSDP".to_string()],
            completed_protocols: vec!["ARP".to_string()],
            is_running: true,
        });

        let (sadp_devices, onvif_devices, ssdp_devices) = tokio::join!(
            // 2000ms (was 800ms) — on networks with 100+ devices all replying to the same
            // broadcast probe, a short window risks losing responses to socket buffer pressure.
            SadpProvider::probe(&broadcast_targets, Duration::from_millis(2000)),
            OnvifProvider::probe(&broadcast_targets, Duration::from_millis(2000)),
            SsdpProvider::probe(&broadcast_targets, Duration::from_millis(2000))
        );

        for dev in sadp_devices {
            deduplicator.insert_or_merge(dev);
        }
        for dev in onvif_devices {
            deduplicator.insert_or_merge(dev);
        }
        for dev in ssdp_devices {
            deduplicator.insert_or_merge(dev);
        }

        // Phase 3: Active Subnet Sweep with Bounded Concurrency (65%)
        // Uses the interface's real netmask (falls back to /24 for unparseable/oversized masks)
        // instead of assuming every network is a /24.
        // Cap raised from 1024 to 4096 (covers up to a /20) — camera VLANs in larger deployments
        // commonly exceed a /22's 1022 usable hosts, and 48-worker bounded concurrency handles a
        // 4096-host sweep in well under a minute.
        let host_ips = NetworkInterfaceManager::host_ips_in_subnet(&selected_iface.ip, &selected_iface.netmask, 4096);

        progress_callback(DiscoveryProgress {
            percentage: 65,
            phase: format!("Varrendo {} endereços na rede {}/{}...", host_ips.len(), selected_iface.ip, selected_iface.netmask),
            devices_found: 0,
            active_protocols: vec!["TCP".to_string(), "RTSP".to_string(), "HTTP".to_string()],
            completed_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string(), "SSDP".to_string()],
            is_running: true,
        });

        let semaphore = Arc::new(Semaphore::new(48));
        let mut tcp_set = JoinSet::new();

        for ip_str in host_ips {
            let sem = semaphore.clone();
            tcp_set.spawn(async move {
                let _permit = sem.acquire().await.ok();
                let open_ports = TcpPortProvider::check_all_ports(&ip_str).await;
                (ip_str, open_ports)
            });
        }

        let mut active_hosts = Vec::new();
        while let Some(res) = tcp_set.join_next().await {
            if let Ok((ip_str, ports)) = res {
                let has_any_port = ports.rtsp_554 || ports.rtsp_8554 || ports.rtsp_10554 || ports.hikvision_8000 || ports.dahua_37777
                    || ports.http_80 || ports.https_443 || ports.http_8080
                    || ports.ssh_22 || ports.smb_445 || ports.postgres_5432 || ports.mysql_3306 || ports.docker_2375
                    || ports.snmp_161 || ports.telnet_23 || ports.dns_53;

                if has_any_port {
                    active_hosts.push((ip_str, ports));
                }
            }
        }

        // Phase 4: Parallel HTTP Fingerprinting (85%)
        progress_callback(DiscoveryProgress {
            percentage: 85,
            phase: "Coletando assinaturas HTTP e títulos de serviços...".to_string(),
            devices_found: active_hosts.len(),
            active_protocols: vec!["HTTP Fingerprint".to_string()],
            completed_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string(), "SSDP".to_string(), "TCP".to_string()],
            is_running: true,
        });

        let http_sem = Arc::new(Semaphore::new(32));
        let mut http_set = JoinSet::new();

        for (ip_str, ports) in active_hosts {
            let sem = http_sem.clone();
            let default_gw = default_gw_ip.clone();
            let arp_mac = arp_table.get(&ip_str).cloned();

            http_set.spawn(async move {
                let _permit = sem.acquire().await.ok();
                let http_fp = if ports.http_80 {
                    HttpFingerprintProvider::fingerprint(&ip_str, 80).await
                } else if ports.http_8080 {
                    HttpFingerprintProvider::fingerprint(&ip_str, 8080).await
                } else if ports.https_443 {
                    HttpFingerprintProvider::fingerprint(&ip_str, 443).await
                } else {
                    None
                };

                let is_gw = !default_gw.is_empty() && ip_str == default_gw;

                let mut protocols = Vec::new();
                if ports.rtsp_554 { protocols.push("RTSP:554".to_string()); }
                if ports.rtsp_8554 { protocols.push("RTSP:8554".to_string()); }
                if ports.rtsp_10554 { protocols.push("RTSP:10554".to_string()); }
                if ports.hikvision_8000 { protocols.push("SDK:8000".to_string()); }
                if ports.dahua_37777 { protocols.push("SDK:37777".to_string()); }
                if ports.http_80 { protocols.push("HTTP:80".to_string()); }
                if ports.https_443 { protocols.push("HTTPS:443".to_string()); }
                if ports.ssh_22 { protocols.push("SSH:22".to_string()); }
                if ports.smb_445 { protocols.push("SMB:445".to_string()); }
                if ports.postgres_5432 { protocols.push("Postgres:5432".to_string()); }
                if ports.mysql_3306 { protocols.push("MySQL:3306".to_string()); }
                if ports.docker_2375 { protocols.push("Docker:2375".to_string()); }
                if ports.snmp_161 { protocols.push("SNMP:161".to_string()); }
                if ports.dns_53 { protocols.push("DNS:53".to_string()); }

                let ctx = ClassificationContext {
                    ip: &ip_str,
                    mac: arp_mac.as_deref(),
                    hardware_model: "",
                    scopes: "",
                    name: "",
                    has_sadp: false,
                    sadp_model: None,
                    has_onvif: false,
                    has_ssdp: false,
                    open_ports: &ports,
                    http_fp: http_fp.as_ref(),
                    is_default_gateway: is_gw,
                };

                let res = classify_device(&ctx);

                DiscoveredDevice {
                    id: arp_mac.clone().unwrap_or_else(|| ip_str.clone()),
                    ip: ip_str.clone(),
                    mac: arp_mac,
                    brand: res.brand,
                    hardware_model: res.hardware_model,
                    name: res.name,
                    device_type: res.device_type,
                    device_type_label: res.device_type_label,
                    serial_number: None,
                    firmware_version: None,
                    activation_status: Some("Ativo".to_string()),
                    rtsp_port: if ports.rtsp_554 { 554 } else if ports.rtsp_8554 { 8554 } else if ports.rtsp_10554 { 10554 } else { 0 },
                    http_port: if ports.http_80 { 80 } else if ports.http_8080 { 8080 } else if ports.https_443 { 443 } else { 0 },
                    sdk_port: if ports.hikvision_8000 { 8000 } else if ports.dahua_37777 { 37777 } else { 0 },
                    protocols,
                    confidence_score: res.confidence_score,
                    confidence_level: res.confidence_level,
                    evidences: res.evidences,
                    contradictions: res.contradictions,
                    issues: Vec::new(),
                    xaddrs: if ports.http_80 { format!("http://{}:80/onvif/device_service", ip_str) } else { String::new() },
                    is_already_added: false,
                }
            });
        }

        while let Some(res) = http_set.join_next().await {
            if let Ok(dev) = res {
                deduplicator.insert_or_merge(dev);
            }
        }

        // Phase 5: Consolidate & Diagnose (100%)
        let mut final_devices = deduplicator.into_vec();

        for dev in &mut final_devices {
            if dev.mac.is_none() {
                if let Some(mac) = arp_table.get(&dev.ip) {
                    dev.mac = Some(mac.clone());
                }
            }

            DiagnosticEngine::diagnose_device(dev, &selected_iface.ip, &selected_iface.netmask);
        }

        progress_callback(DiscoveryProgress {
            percentage: 100,
            phase: "Classificação por evidências concluída!".to_string(),
            devices_found: final_devices.len(),
            active_protocols: Vec::new(),
            completed_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string(), "SSDP".to_string(), "TCP".to_string(), "HTTP Fingerprint".to_string()],
            is_running: false,
        });

        final_devices
    }
}
