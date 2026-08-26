use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::net::{UdpSocket, TcpStream};
use tokio::task::JoinSet;
use uuid::Uuid;
use crate::camera::model::{DiscoveredDevice, DeviceType};

pub struct OnvifDiscovery;

impl OnvifDiscovery {
    pub async fn discover_devices(_timeout_duration: Duration) -> Result<Vec<DiscoveredDevice>, String> {
        let mut discovered_map: HashMap<String, DiscoveredDevice> = HashMap::new();

        // 1. Detect local subnets (e.g. 172.20.120.x)
        let local_subnets = get_local_ipv4_subnets();

        // 2. Prepare ONVIF WS-Discovery probe
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

        // 3. Prepare Hikvision SADP probe
        let sadp_probe_xml = r#"<?xml version="1.0" encoding="utf-8"?><Probe><Types>inquiry</Types></Probe>"#;

        // Open UDP socket for Multicast & Broadcast
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await {
            let _ = socket.set_broadcast(true);

            let mut onvif_targets = vec![
                "239.255.255.250:3702".to_string(),
                "255.255.255.255:3702".to_string(),
            ];
            let mut sadp_targets = vec![
                "239.255.255.250:37020".to_string(),
                "255.255.255.255:37020".to_string(),
            ];

            for (prefix, _) in &local_subnets {
                onvif_targets.push(format!("{}.255:3702", prefix));
                sadp_targets.push(format!("{}.255:37020", prefix));
            }

            for target in onvif_targets {
                if let Ok(addr) = target.parse::<SocketAddr>() {
                    let _ = socket.send_to(ws_probe_xml.as_bytes(), addr).await;
                }
            }

            for target in sadp_targets {
                if let Ok(addr) = target.parse::<SocketAddr>() {
                    let _ = socket.send_to(sadp_probe_xml.as_bytes(), addr).await;
                }
            }

            // Listen for UDP responses for up to 1.5s
            let mut buf = [0u8; 65535];
            let udp_deadline = tokio::time::Instant::now() + Duration::from_millis(1500);

            while tokio::time::Instant::now() < udp_deadline {
                let remaining = udp_deadline - tokio::time::Instant::now();
                match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                    Ok(Ok((len, addr))) => {
                        let xml_str = String::from_utf8_lossy(&buf[..len]);
                        if let Some(device) = parse_probe_matches(&xml_str, addr.ip().to_string()) {
                            discovered_map.insert(device.ip.clone(), device);
                        } else if let Some(device) = parse_sadp_matches(&xml_str, addr.ip().to_string()) {
                            discovered_map.entry(device.ip.clone())
                                .and_modify(|existing| {
                                    if existing.hardware_model == "IP Camera" && device.hardware_model != "IP Camera" {
                                        existing.hardware_model = device.hardware_model.clone();
                                        existing.name = device.name.clone();
                                    }
                                })
                                .or_insert(device);
                        }
                    }
                    _ => break,
                }
            }
        }

        // 4. Concurrent TCP Subnet Sweep on local /24 subnets for RTSP 554, Hikvision 8000, HTTP 80
        for (prefix, _) in &local_subnets {
            let mut join_set = JoinSet::new();
            for i in 1..=254 {
                let ip_str = format!("{}.{}", prefix, i);
                if discovered_map.contains_key(&ip_str) {
                    continue;
                }
                join_set.spawn(async move {
                    probe_cctv_ip(&ip_str).await
                });
            }

            while let Some(res) = join_set.join_next().await {
                if let Ok(Some(device)) = res {
                    discovered_map.entry(device.ip.clone()).or_insert(device);
                }
            }
        }

        let mut list: Vec<DiscoveredDevice> = discovered_map.into_values().collect();
        list.sort_by(|a, b| a.ip.cmp(&b.ip));
        Ok(list)
    }
}

async fn probe_cctv_ip(ip: &str) -> Option<DiscoveredDevice> {
    // Check RTSP port 554 first (fastest)
    let rtsp_open = is_port_open(ip, 554).await;
    let hik_open = is_port_open(ip, 8000).await;
    let http_open = if !rtsp_open && !hik_open {
        is_port_open(ip, 80).await
    } else {
        true
    };

    if !rtsp_open && !hik_open && !http_open {
        return None;
    }

    // Determine brand and model if possible
    let mut brand = "Câmera IP".to_string();
    let mut model = "IP Camera".to_string();

    if hik_open {
        brand = "Hikvision".to_string();
        model = "Hikvision IP Device".to_string();
    }

    let (device_type, device_type_label) = infer_device_type(&model, "", ip);

    Some(DiscoveredDevice {
        ip: ip.to_string(),
        name: format!("{} ({})", brand, ip),
        hardware_model: model,
        brand,
        device_type,
        device_type_label,
        xaddrs: format!("http://{}:80/onvif/device_service", ip),
        rtsp_port: 554,
        is_already_added: false,
    })
}

async fn is_port_open(ip: &str, port: u16) -> bool {
    let addr = format!("{}:{}", ip, port);
    tokio::time::timeout(Duration::from_millis(250), TcpStream::connect(&addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

fn get_local_ipv4_subnets() -> Vec<(String, Ipv4Addr)> {
    let mut subnets = Vec::new();
    if let Ok(output) = std::process::Command::new("ip").args(["-4", "addr"]).output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("inet ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let ip_cidr = parts[1];
                    if let Some(pos) = ip_cidr.find('/') {
                        let ip_str = &ip_cidr[..pos];
                        if let Ok(ipv4) = ip_str.parse::<Ipv4Addr>() {
                            if !ipv4.is_loopback() && !ipv4.is_link_local() {
                                let octets = ipv4.octets();
                                let prefix = format!("{}.{}.{}", octets[0], octets[1], octets[2]);
                                subnets.push((prefix, ipv4));
                            }
                        }
                    }
                }
            }
        }
    }
    if subnets.is_empty() {
        subnets.push(("172.20.120".to_string(), Ipv4Addr::new(172, 20, 120, 1)));
    }
    subnets
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

    let ip = if let Some(extracted_ip) = extract_ip_from_url(&xaddrs) {
        extracted_ip
    } else {
        sender_ip
    };

    let scopes = extract_tag_content(xml, "Scopes")
        .or_else(|| extract_tag_content(xml, "d:Scopes"))
        .or_else(|| extract_tag_content(xml, "wsd:Scopes"))
        .unwrap_or_default();

    let (name, model, brand) = parse_scopes(&scopes, &ip);
    let (device_type, device_type_label) = infer_device_type(&model, &scopes, &name);

    Some(DiscoveredDevice {
        ip,
        name,
        hardware_model: model,
        brand,
        device_type,
        device_type_label,
        xaddrs,
        rtsp_port: 554,
        is_already_added: false,
    })
}

pub fn parse_sadp_matches(xml: &str, sender_ip: String) -> Option<DiscoveredDevice> {
    if !xml.contains("<ProbeMatch>") && !xml.contains("DeviceDescription") {
        return None;
    }

    let ip = extract_tag_content(xml, "IPv4Address")
        .unwrap_or(sender_ip);

    let model = extract_tag_content(xml, "DeviceDescription")
        .unwrap_or_else(|| "Hikvision Device".to_string());

    let name = format!("Hikvision {}", model);
    let (device_type, device_type_label) = infer_device_type(&model, "", &name);

    Some(DiscoveredDevice {
        ip,
        name,
        hardware_model: model,
        brand: "Hikvision".to_string(),
        device_type,
        device_type_label,
        xaddrs: "".to_string(),
        rtsp_port: 554,
        is_already_added: false,
    })
}

pub fn infer_device_type(hardware_model: &str, scopes: &str, name: &str) -> (DeviceType, String) {
    let m = hardware_model.to_uppercase();
    let s = scopes.to_lowercase();
    let n = name.to_uppercase();

    // 1. Tráfego / Leitura de Placa (LPR / ANPR) / Radar
    if m.starts_with("DS-TCG") || m.starts_with("DS-TPE") || m.starts_with("IDS-2CD9") 
        || m.starts_with("IDS-T") || m.starts_with("ITC") || m.contains("LPR") 
        || m.contains("ANPR") || s.contains("traffic") || s.contains("anpr") 
        || s.contains("lpr") || s.contains("radar") || s.contains("checkpoint") 
        || n.contains("LPR") || n.contains("TRAFEGO") || n.contains("RADAR") {
        return (DeviceType::TrafficLpr, "Câmera de Tráfego / LPR".to_string());
    }

    // 2. Intercom / Videoporteiro / Comunicação / Controle de Acesso
    if m.starts_with("DS-KB") || m.starts_with("DS-KD") || m.starts_with("DS-KH") 
        || m.starts_with("DS-KV") || m.starts_with("DS-K1") || m.starts_with("DS-K2")
        || m.starts_with("VTO") || m.starts_with("VTH") || m.starts_with("PVIP")
        || m.starts_with("ALLO") || m.contains("DOOR") || m.contains("INTERCOM")
        || s.contains("intercom") || s.contains("door") || s.contains("access_control")
        || n.contains("INTERFONE") || n.contains("VIDEOPORTEIRO") || n.contains("CAMPANHIA")
        || n.contains("PORTAO") {
        return (DeviceType::Intercom, "Videoporteiro / Comunicação".to_string());
    }

    // 3. NVR / DVR / Gravador de Vídeo
    if m.starts_with("DS-76") || m.starts_with("DS-77") || m.starts_with("DS-96") 
        || m.starts_with("DS-71") || m.starts_with("DS-72") || m.starts_with("DS-73") 
        || m.starts_with("DS-81") || m.starts_with("IDS-76") || m.starts_with("IDS-77") 
        || m.starts_with("IDS-96") || m.starts_with("NVD") || m.starts_with("MHDX") 
        || m.starts_with("NVR") || m.starts_with("XVR") || m.starts_with("HCVR")
        || s.contains("profile/g") || s.contains("recording") || s.contains("nvr") || s.contains("dvr") {
        return (DeviceType::Nvr, "NVR / Gravador".to_string());
    }

    // 4. Câmera PTZ / Speed Dome
    if m.starts_with("DS-2DE") || m.starts_with("DS-2DF") || m.starts_with("DS-2DY") 
        || m.starts_with("IDS-2DE") || m.starts_with("SD") || m.contains(" PTZ") 
        || m.contains("SPEED") || s.contains("type/ptz") || s.contains("ptz") || s.contains("speeddome") {
        return (DeviceType::Ptz, "Câmera PTZ / Speed Dome".to_string());
    }

    // 5. Câmera Térmica
    if m.starts_with("DS-2TD") || s.contains("thermal") || s.contains("thermography") || m.contains("THERMAL") {
        return (DeviceType::Thermal, "Câmera Térmica".to_string());
    }

    // 6. Câmera IP Convencional (Bullet, Dome, Turret)
    if m.starts_with("DS-2CD") || m.starts_with("DS-2CV") || m.starts_with("IDS-2CD") 
        || m.starts_with("VIP") || m.starts_with("IPC") || m.starts_with("DH-IPC") 
        || s.contains("video_encoder") || s.contains("streaming") {
        return (DeviceType::IpCamera, "Câmera IP".to_string());
    }

    (DeviceType::Other, "Dispositivo CFTV".to_string())
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
