use std::time::Duration;
use tokio::task::JoinSet;
use crate::discovery::types::{DiscoveredDevice, NetworkInterfaceInfo, DiscoveryProgress};
use crate::discovery::network_interfaces::NetworkInterfaceManager;
use crate::discovery::providers::sadp::SadpProvider;
use crate::discovery::providers::onvif::OnvifProvider;
use crate::discovery::providers::arp::ArpProvider;
use crate::discovery::providers::tcp::TcpPortProvider;
use crate::discovery::providers::http::HttpFingerprintProvider;
use crate::discovery::deduplicator::Deduplicator;
use crate::discovery::diagnostic::DiagnosticEngine;
use crate::discovery::classifier::infer_device_type;

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

        // Phase 2: SADP + ONVIF UDP Broadcasts (40%)
        progress_callback(DiscoveryProgress {
            percentage: 30,
            phase: "Enviando sondas ONVIF WS-Discovery e Hikvision SADP...".to_string(),
            devices_found: 0,
            active_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string()],
            completed_protocols: vec!["ARP".to_string()],
            is_running: true,
        });

        let (sadp_devices, onvif_devices) = tokio::join!(
            SadpProvider::probe(&broadcast_targets, Duration::from_millis(1200)),
            OnvifProvider::probe(&broadcast_targets, Duration::from_millis(1200))
        );

        for dev in sadp_devices {
            deduplicator.insert_or_merge(dev);
        }
        for dev in onvif_devices {
            deduplicator.insert_or_merge(dev);
        }

        // Phase 3: Active Subnet Sweep on selected interface prefix (75%)
        let subnet_prefix = selected_iface.ip.rsplit_once('.').map(|(p, _)| p).unwrap_or("172.20.120");

        progress_callback(DiscoveryProgress {
            percentage: 60,
            phase: format!("Varrendo sub-rede {}/24 em portas RTSP e HTTP...", selected_iface.ip),
            devices_found: 0,
            active_protocols: vec!["TCP".to_string(), "RTSP".to_string(), "HTTP".to_string()],
            completed_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string()],
            is_running: true,
        });

        let mut tcp_set = JoinSet::new();
        for i in 1..=254 {
            let ip_str = format!("{}.{}", subnet_prefix, i);
            tcp_set.spawn(async move {
                let open_ports = TcpPortProvider::check_cctv_ports(&ip_str).await;
                (ip_str, open_ports)
            });
        }

        while let Some(res) = tcp_set.join_next().await {
            if let Ok((ip_str, ports)) = res {
                if ports.rtsp_554 || ports.hikvision_8000 || ports.dahua_37777 || ports.http_80 {
                    let mut protocols = Vec::new();
                    if ports.rtsp_554 { protocols.push("RTSP".to_string()); }
                    if ports.hikvision_8000 { protocols.push("SDK:8000".to_string()); }
                    if ports.dahua_37777 { protocols.push("SDK:37777".to_string()); }
                    if ports.http_80 { protocols.push("HTTP".to_string()); }

                    let mut brand = "Câmera IP".to_string();
                    let mut model = "IP Camera".to_string();

                    if ports.hikvision_8000 {
                        brand = "Hikvision".to_string();
                        model = "Hikvision IP Device".to_string();
                    } else if ports.dahua_37777 {
                        brand = "Dahua/Intelbras".to_string();
                        model = "Câmera / Gravador IP".to_string();
                    }

                    // Check ARP for MAC
                    let mac = arp_table.get(&ip_str).cloned();
                    if let Some(ref m) = mac {
                        if let Some(oui_vendor) = ArpProvider::lookup_oui_vendor(m) {
                            brand = oui_vendor;
                        }
                    }

                    let (device_type, device_type_label) = infer_device_type(&model, "", &ip_str);

                    deduplicator.insert_or_merge(DiscoveredDevice {
                        id: mac.clone().unwrap_or_else(|| ip_str.clone()),
                        ip: ip_str.clone(),
                        mac,
                        brand,
                        hardware_model: model,
                        name: format!("Câmera ({})", ip_str),
                        device_type,
                        device_type_label,
                        serial_number: None,
                        firmware_version: None,
                        activation_status: Some("Ativo".to_string()),
                        rtsp_port: if ports.rtsp_554 { 554 } else { 0 },
                        http_port: if ports.http_80 { 80 } else { 0 },
                        sdk_port: if ports.hikvision_8000 { 8000 } else if ports.dahua_37777 { 37777 } else { 0 },
                        protocols,
                        confidence_score: 70,
                        issues: Vec::new(),
                        xaddrs: format!("http://{}:80/onvif/device_service", ip_str),
                        is_already_added: false,
                    });
                }
            }
        }

        // Phase 4: HTTP Fingerprinting on candidates with HTTP open (90%)
        progress_callback(DiscoveryProgress {
            percentage: 85,
            phase: "Analisando assinaturas HTTP e diagnóstico de portas...".to_string(),
            devices_found: 0,
            active_protocols: vec!["HTTP Fingerprint".to_string()],
            completed_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string(), "TCP".to_string()],
            is_running: true,
        });

        // Consolidate list and run HTTP Fingerprint + Diagnostics
        let mut final_devices = deduplicator.into_vec();

        for dev in &mut final_devices {
            // Fill MAC from ARP if missing
            if dev.mac.is_none() {
                if let Some(mac) = arp_table.get(&dev.ip) {
                    dev.mac = Some(mac.clone());
                    if dev.brand == "Câmera IP" || dev.brand == "ONVIF Camera" {
                        if let Some(v) = ArpProvider::lookup_oui_vendor(mac) {
                            dev.brand = v;
                        }
                    }
                }
            }

            // HTTP Fingerprint if model is generic
            if (dev.hardware_model == "IP Camera" || dev.hardware_model == "Hikvision IP Device") && dev.http_port == 80 {
                if let Some(fp) = HttpFingerprintProvider::fingerprint(&dev.ip, dev.http_port).await {
                    if fp.is_hikvision {
                        dev.brand = "Hikvision".to_string();
                        if !dev.protocols.contains(&"HTTP".to_string()) {
                            dev.protocols.push("HTTP".to_string());
                        }
                    } else if fp.is_dahua {
                        dev.brand = "Dahua".to_string();
                    } else if fp.is_intelbras {
                        dev.brand = "Intelbras".to_string();
                    }
                }
            }

            // Run Diagnostic Engine
            DiagnosticEngine::diagnose_device(dev, &selected_iface.ip, &selected_iface.netmask);
        }

        // Phase 5: Final completion (100%)
        progress_callback(DiscoveryProgress {
            percentage: 100,
            phase: "Descoberta inteligente concluída!".to_string(),
            devices_found: final_devices.len(),
            active_protocols: Vec::new(),
            completed_protocols: vec!["ARP".to_string(), "SADP".to_string(), "ONVIF".to_string(), "TCP".to_string(), "HTTP".to_string()],
            is_running: false,
        });

        final_devices
    }
}
