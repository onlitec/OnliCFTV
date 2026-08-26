use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use crate::discovery::types::DiscoveredDevice;
use crate::discovery::classifier::{classify_device, ClassificationContext};
use crate::discovery::providers::tcp::OpenPorts;

pub struct SadpProvider;

#[derive(Debug, Clone)]
pub struct SadpDeviceRecord {
    pub ip: String,
    pub mac: Option<String>,
    pub model: String,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub activated: Option<bool>,
    pub http_port: u16,
    pub sdk_port: u16,
    pub subnet_mask: Option<String>,
    pub gateway: Option<String>,
}

impl SadpProvider {
    pub async fn probe(broadcast_ips: &[String], timeout: Duration) -> Vec<DiscoveredDevice> {
        let mut results = Vec::new();
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return results,
        };

        let _ = socket.set_broadcast(true);

        let sadp_xml = r#"<?xml version="1.0" encoding="utf-8"?><Probe><Types>inquiry</Types></Probe>"#;
        let sadp_bytes = sadp_xml.as_bytes();

        let mut targets = vec![
            "239.255.255.250:37020".to_string(),
            "255.255.255.255:37020".to_string(),
        ];
        for b in broadcast_ips {
            targets.push(format!("{}:37020", b));
        }

        for target in targets {
            if let Ok(addr) = target.parse::<SocketAddr>() {
                let _ = socket.send_to(sadp_bytes, addr).await;
            }
        }

        let mut buf = vec![0u8; 8192];
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    let xml_str = String::from_utf8_lossy(&buf[..len]);
                    if let Some(record) = parse_sadp_xml(&xml_str, addr.ip().to_string()) {
                        let open_ports = OpenPorts {
                            rtsp_554: true,
                            hikvision_8000: record.sdk_port == 8000,
                            http_80: record.http_port == 80,
                            ..Default::default()
                        };

                        let ctx = ClassificationContext {
                            ip: &record.ip,
                            mac: record.mac.as_deref(),
                            hardware_model: &record.model,
                            scopes: "",
                            name: &format!("Hikvision {}", record.model),
                            has_sadp: true,
                            sadp_model: Some(&record.model),
                            has_onvif: false,
                            open_ports: &open_ports,
                            http_fp: None,
                            is_default_gateway: false,
                        };

                        let res = classify_device(&ctx);

                        let act_status = match record.activated {
                            Some(true) => "Ativo".to_string(),
                            Some(false) => "Aguardando ativação".to_string(),
                            None => "Ativo".to_string(),
                        };

                        results.push(DiscoveredDevice {
                            id: record.mac.clone().unwrap_or_else(|| record.ip.clone()),
                            ip: record.ip.clone(),
                            mac: record.mac,
                            brand: res.brand,
                            hardware_model: record.model.clone(),
                            name: format!("Hikvision {}", record.model),
                            device_type: res.device_type,
                            device_type_label: res.device_type_label,
                            serial_number: record.serial,
                            firmware_version: record.firmware,
                            activation_status: Some(act_status),
                            rtsp_port: 554,
                            http_port: record.http_port,
                            sdk_port: record.sdk_port,
                            protocols: vec!["SADP".to_string()],
                            confidence_score: res.confidence_score,
                            confidence_level: res.confidence_level,
                            evidences: res.evidences,
                            contradictions: res.contradictions,
                            issues: Vec::new(),
                            xaddrs: String::new(),
                            is_already_added: false,
                        });
                    }
                }
                _ => break,
            }
        }

        results
    }
}

pub fn parse_sadp_xml(xml: &str, fallback_ip: String) -> Option<SadpDeviceRecord> {
    if !xml.contains("<ProbeMatch>") && !xml.contains("DeviceDescription") {
        return None;
    }

    let ip = extract_xml_tag(xml, "IPv4Address").unwrap_or(fallback_ip);
    let model = extract_xml_tag(xml, "DeviceDescription").unwrap_or_else(|| "Hikvision IP Device".to_string());
    let mac = extract_xml_tag(xml, "MAC").map(|m| m.to_lowercase().replace('-', ":"));
    let serial = extract_xml_tag(xml, "DeviceSN");
    let firmware = extract_xml_tag(xml, "SoftwareVersion");
    let http_port: u16 = extract_xml_tag(xml, "HttpPort").and_then(|p| p.parse().ok()).unwrap_or(80);
    let sdk_port: u16 = extract_xml_tag(xml, "CommandPort").and_then(|p| p.parse().ok()).unwrap_or(8000);
    let subnet_mask = extract_xml_tag(xml, "IPv4SubnetMask");
    let gateway = extract_xml_tag(xml, "IPv4Gateway");
    
    let activated = extract_xml_tag(xml, "Activated").map(|a| a.to_lowercase() == "true");

    Some(SadpDeviceRecord {
        ip,
        mac,
        model,
        serial,
        firmware,
        activated,
        http_port,
        sdk_port,
        subnet_mask,
        gateway,
    })
}

pub fn extract_xml_tag(xml: &str, tag_name: &str) -> Option<String> {
    let open_tag = format!("<{}", tag_name);
    let close_tag = format!("</{}", tag_name);

    let start_pos = xml.find(&open_tag)?;
    let after_open = &xml[start_pos..];
    let tag_end = after_open.find('>')?;
    let content_start = start_pos + tag_end + 1;

    let end_pos = xml[content_start..].find(&close_tag)?;
    let content = &xml[content_start..content_start + end_pos];
    Some(content.trim().to_string())
}
