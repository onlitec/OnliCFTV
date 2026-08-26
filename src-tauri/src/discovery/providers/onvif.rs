use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use uuid::Uuid;
use crate::discovery::types::DiscoveredDevice;
use crate::discovery::classifier::{classify_device, ClassificationContext};
use crate::discovery::providers::tcp::OpenPorts;
use crate::discovery::providers::sadp::extract_xml_tag;

pub struct OnvifProvider;

impl OnvifProvider {
    pub async fn probe(broadcast_ips: &[String], timeout: Duration) -> Vec<DiscoveredDevice> {
        let mut results = Vec::new();
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return results,
        };

        let _ = socket.set_broadcast(true);

        let probe_id = Uuid::new_v4().to_string();
        let ws_probe_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<e:Envelope xmlns:e="http://www.w3.org/2003/05/soap-envelope"
            xmlns:w="http://schemas.xmlsoap.org/ws/2004/08/addressing"
            xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"
            xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
  <e:Header>
    <w:MessageID>uuid:{}</w:MessageID>
    <w:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</w:To>
    <w:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</w:Action>
  </e:Header>
  <e:Body>
    <d:Probe>
      <d:Types>dn:NetworkVideoTransmitter</d:Types>
    </d:Probe>
  </e:Body>
</e:Envelope>"#,
            probe_id
        );
        let probe_bytes = ws_probe_xml.as_bytes();

        let mut targets = vec![
            "239.255.255.250:3702".to_string(),
            "255.255.255.255:3702".to_string(),
        ];
        for b in broadcast_ips {
            targets.push(format!("{}:3702", b));
        }

        for target in targets {
            if let Ok(addr) = target.parse::<SocketAddr>() {
                let _ = socket.send_to(probe_bytes, addr).await;
            }
        }

        let mut buf = vec![0u8; 8192];
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    let xml_str = String::from_utf8_lossy(&buf[..len]);
                    if let Some(device) = parse_onvif_xml(&xml_str, addr.ip().to_string()) {
                        results.push(device);
                    }
                }
                _ => break,
            }
        }

        results
    }
}

pub fn parse_onvif_xml(xml: &str, fallback_ip: String) -> Option<DiscoveredDevice> {
    if !xml.contains("ProbeMatches") && !xml.contains("ProbeMatch") {
        return None;
    }

    let xaddrs = extract_xml_tag(xml, "XAddrs")
        .or_else(|| extract_xml_tag(xml, "d:XAddrs"))
        .or_else(|| extract_xml_tag(xml, "wsd:XAddrs"))
        .unwrap_or_default();

    let ip = if let Some(extracted_ip) = extract_ip_from_xaddrs(&xaddrs) {
        extracted_ip
    } else {
        fallback_ip
    };

    let scopes = extract_xml_tag(xml, "Scopes")
        .or_else(|| extract_xml_tag(xml, "d:Scopes"))
        .or_else(|| extract_xml_tag(xml, "wsd:Scopes"))
        .unwrap_or_default();

    let (name, model, brand) = parse_onvif_scopes(&scopes, &ip);
    
    let open_ports = OpenPorts {
        rtsp_554: true,
        http_80: true,
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: &ip,
        mac: None,
        hardware_model: &model,
        scopes: &scopes,
        name: &name,
        has_sadp: false,
        sadp_model: None,
        has_onvif: true,
        open_ports: &open_ports,
        http_fp: None,
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);

    Some(DiscoveredDevice {
        id: ip.clone(),
        ip,
        mac: None,
        brand: if res.brand != "Dispositivo de Rede" { res.brand } else { brand },
        hardware_model: model,
        name,
        device_type: res.device_type,
        device_type_label: res.device_type_label,
        serial_number: None,
        firmware_version: None,
        activation_status: Some("Ativo".to_string()),
        rtsp_port: 554,
        http_port: 80,
        sdk_port: 8000,
        protocols: vec!["ONVIF".to_string()],
        confidence_score: res.confidence_score,
        confidence_level: res.confidence_level,
        evidences: res.evidences,
        contradictions: res.contradictions,
        issues: Vec::new(),
        xaddrs,
        is_already_added: false,
    })
}

fn extract_ip_from_xaddrs(url: &str) -> Option<String> {
    for word in url.split_whitespace() {
        if let Some(pos) = word.find("://") {
            let host_part = &word[pos + 3..];
            let host_end = host_part.find('/').unwrap_or(host_part.len());
            let host_port = &host_part[..host_end];
            let ip = if let Some(colon) = host_port.find(':') {
                &host_port[..colon]
            } else {
                host_port
            };
            if !ip.is_empty() && ip.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return Some(ip.to_string());
            }
        }
    }
    None
}

fn parse_onvif_scopes(scopes: &str, fallback_ip: &str) -> (String, String, String) {
    let mut name = None;
    let mut model = None;
    let mut brand = "ONVIF Device".to_string();

    let scopes_lower = scopes.to_lowercase();
    if scopes_lower.contains("hikvision") {
        brand = "Hikvision".to_string();
    } else if scopes_lower.contains("intelbras") {
        brand = "Intelbras".to_string();
    } else if scopes_lower.contains("dahua") {
        brand = "Dahua".to_string();
    } else if scopes_lower.contains("axis") {
        brand = "Axis".to_string();
    }

    for item in scopes.split_whitespace() {
        let clean = item.trim();
        if clean.contains("/name/") {
            if let Some(pos) = clean.find("/name/") {
                let raw_name = &clean[pos + 6..];
                name = Some(url_decode(raw_name));
            }
        } else if clean.contains("/hardware/") {
            if let Some(pos) = clean.find("/hardware/") {
                let raw_model = &clean[pos + 10..];
                model = Some(url_decode(raw_model));
            }
        }
    }

    let final_model = model.unwrap_or_else(|| "IP Camera".to_string());
    let final_name = name.unwrap_or_else(|| format!("{} ({})", final_model, fallback_ip));

    (final_name, final_model, brand)
}

fn url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                let hex_str = format!("{}{}", c1, c2);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}
