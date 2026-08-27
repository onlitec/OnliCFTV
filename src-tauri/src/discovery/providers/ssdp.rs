use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use crate::discovery::types::DiscoveredDevice;
use crate::discovery::classifier::{classify_device, ClassificationContext};
use crate::discovery::providers::tcp::OpenPorts;

pub struct SsdpProvider;

impl SsdpProvider {
    /// Sends an SSDP (UPnP) M-SEARCH probe and collects responses for `timeout`. Catches generic
    /// IP cameras/NVRs that advertise themselves via UPnP but weren't caught by SADP or ONVIF
    /// WS-Discovery (e.g. generic/no-name ODM cameras).
    pub async fn probe(broadcast_ips: &[String], timeout: Duration) -> Vec<DiscoveredDevice> {
        let mut results = Vec::new();
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return results,
        };
        let _ = socket.set_broadcast(true);

        let request = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: ssdp:all\r\n\r\n";
        let request_bytes = request.as_bytes();

        let mut targets = vec!["239.255.255.250:1900".to_string()];
        for b in broadcast_ips {
            targets.push(format!("{}:1900", b));
        }

        for target in targets {
            if let Ok(addr) = target.parse::<SocketAddr>() {
                let _ = socket.send_to(request_bytes, addr).await;
            }
        }

        let mut buf = vec![0u8; 4096];
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    let response = String::from_utf8_lossy(&buf[..len]);
                    if let Some(device) = parse_ssdp_response(&response, addr.ip().to_string()) {
                        results.push(device);
                    }
                }
                _ => break,
            }
        }

        results
    }
}

fn get_header<'a>(response: &'a str, name: &str) -> Option<&'a str> {
    for line in response.lines() {
        if let Some(colon) = line.find(':') {
            if line[..colon].trim().eq_ignore_ascii_case(name) {
                return Some(line[colon + 1..].trim());
            }
        }
    }
    None
}

fn extract_ip_from_location(location: &str) -> Option<String> {
    let after_scheme = location.split("://").nth(1)?;
    let host_port = after_scheme.split('/').next()?;
    let ip = host_port.split(':').next()?;
    if !ip.is_empty() && ip.chars().all(|c| c.is_ascii_digit() || c == '.') {
        Some(ip.to_string())
    } else {
        None
    }
}

pub fn parse_ssdp_response(response: &str, fallback_ip: String) -> Option<DiscoveredDevice> {
    let upper = response.to_uppercase();
    if !upper.starts_with("HTTP/1.1 200") && !upper.starts_with("HTTP/1.0 200") {
        return None;
    }

    let location = get_header(response, "location").unwrap_or_default();
    let server = get_header(response, "server").unwrap_or_default();
    let st = get_header(response, "st").unwrap_or_default();

    let ip = extract_ip_from_location(location).unwrap_or(fallback_ip);
    let combined = format!("{} {}", server, st).to_lowercase();

    let mut brand_hint = "Dispositivo de Rede".to_string();
    if combined.contains("hikvision") {
        brand_hint = "Hikvision".to_string();
    } else if combined.contains("dahua") {
        brand_hint = "Dahua".to_string();
    } else if combined.contains("intelbras") {
        brand_hint = "Intelbras".to_string();
    } else if combined.contains("axis") {
        brand_hint = "Axis".to_string();
    } else if combined.contains("uniview") {
        brand_hint = "Uniview".to_string();
    } else if combined.contains("reolink") {
        brand_hint = "Reolink".to_string();
    } else if combined.contains("vivotek") {
        brand_hint = "Vivotek".to_string();
    } else if combined.contains("bosch") {
        brand_hint = "Bosch".to_string();
    } else if combined.contains("hanwha") || combined.contains("wisenet") {
        brand_hint = "Hanwha".to_string();
    } else if combined.contains("tp-link") || combined.contains("vigi") {
        brand_hint = "TP-Link".to_string();
    }

    let open_ports = OpenPorts {
        http_80: true,
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: &ip,
        mac: None,
        hardware_model: "",
        scopes: &combined,
        name: "",
        has_sadp: false,
        sadp_model: None,
        has_onvif: false,
        has_ssdp: true,
        open_ports: &open_ports,
        http_fp: None,
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);

    Some(DiscoveredDevice {
        id: ip.clone(),
        ip: ip.clone(),
        mac: None,
        brand: if res.brand != "Dispositivo de Rede" { res.brand } else { brand_hint },
        hardware_model: res.hardware_model,
        name: res.name,
        device_type: res.device_type,
        device_type_label: res.device_type_label,
        serial_number: None,
        firmware_version: None,
        activation_status: Some("Ativo".to_string()),
        rtsp_port: 0,
        http_port: 80,
        sdk_port: 0,
        protocols: vec!["SSDP/UPnP".to_string()],
        confidence_score: res.confidence_score,
        confidence_level: res.confidence_level,
        evidences: res.evidences,
        contradictions: res.contradictions,
        issues: Vec::new(),
        xaddrs: location.to_string(),
        is_already_added: false,
    })
}
