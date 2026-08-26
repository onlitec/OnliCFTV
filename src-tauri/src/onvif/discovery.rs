use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use uuid::Uuid;
use crate::camera::model::DiscoveredDevice;

pub struct OnvifDiscovery;

impl OnvifDiscovery {
    pub async fn discover_devices(timeout_duration: Duration) -> Result<Vec<DiscoveredDevice>, String> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Falha ao abrir socket UDP para descoberta ONVIF: {}", e))?;

        socket.set_broadcast(true)
            .map_err(|e| format!("Falha ao habilitar broadcast UDP: {}", e))?;

        let probe_id = Uuid::new_v4().to_string();
        let probe_xml = format!(
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

        let probe_bytes = probe_xml.as_bytes();

        // Send to ONVIF multicast and local subnet broadcast addresses
        let targets = [
            "239.255.255.250:3702",
            "255.255.255.255:3702",
            "172.20.120.255:3702",
            "192.168.1.255:3702",
            "192.168.0.255:3702",
            "10.0.0.255:3702",
        ];

        for target in targets {
            if let Ok(addr) = target.parse::<SocketAddr>() {
                let _ = socket.send_to(probe_bytes, addr).await;
            }
        }

        let mut discovered_map: HashMap<String, DiscoveredDevice> = HashMap::new();
        let mut buf = [0u8; 65535];
        let start_time = tokio::time::Instant::now();

        loop {
            let elapsed = start_time.elapsed();
            if elapsed >= timeout_duration {
                break;
            }
            let remaining = timeout_duration - elapsed;

            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    let xml_str = String::from_utf8_lossy(&buf[..len]);
                    if let Some(device) = parse_probe_matches(&xml_str, addr.ip().to_string()) {
                        discovered_map.entry(device.ip.clone()).or_insert(device);
                    }
                }
                Ok(Err(_)) => break,
                Err(_) => break, // Timeout
            }
        }

        let mut list: Vec<DiscoveredDevice> = discovered_map.into_values().collect();
        list.sort_by(|a, b| a.ip.cmp(&b.ip));
        Ok(list)
    }
}

pub fn parse_probe_matches(xml: &str, sender_ip: String) -> Option<DiscoveredDevice> {
    if !xml.contains("ProbeMatches") && !xml.contains("ProbeMatch") {
        return None;
    }

    // Extract XAddrs
    let xaddrs = extract_tag_content(xml, "XAddrs")
        .or_else(|| extract_tag_content(xml, "d:XAddrs"))
        .or_else(|| extract_tag_content(xml, "wsd:XAddrs"))
        .unwrap_or_default();

    // Extract IP from XAddrs (e.g. "http://172.20.120.67:80/onvif/device_service") or fallback to sender_ip
    let ip = if let Some(extracted_ip) = extract_ip_from_url(&xaddrs) {
        extracted_ip
    } else {
        sender_ip
    };

    // Extract Scopes
    let scopes = extract_tag_content(xml, "Scopes")
        .or_else(|| extract_tag_content(xml, "d:Scopes"))
        .or_else(|| extract_tag_content(xml, "wsd:Scopes"))
        .unwrap_or_default();

    let (name, model, brand) = parse_scopes(&scopes, &ip);

    Some(DiscoveredDevice {
        ip,
        name,
        hardware_model: model,
        brand,
        xaddrs,
        rtsp_port: 554,
        is_already_added: false,
    })
}

fn extract_tag_content(xml: &str, tag_name: &str) -> Option<String> {
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

fn extract_ip_from_url(url: &str) -> Option<String> {
    // Looks for http://172.20.120.67:80/... or http://172.20.120.67/...
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

fn parse_scopes(scopes: &str, fallback_ip: &str) -> (String, String, String) {
    let mut name = None;
    let mut model = None;
    let mut brand = "ONVIF Camera".to_string();

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
