use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Mutex;
use md5::{Md5, Digest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use serde::{Serialize, Deserialize};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use uuid::Uuid;

use crate::discovery::providers::sadp::extract_xml_tag;

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Extracts the <name> value scoped to <channelNameOverlay>, instead of the first <name> tag
/// anywhere in the VideoOverlay document (which may belong to an unrelated overlay slot).
fn extract_channel_name_overlay_value(xml: &str) -> Option<String> {
    let start = xml.find("<channelNameOverlay")?;
    let rel_end = xml[start..].find("</channelNameOverlay>")?;
    let end = start + rel_end + "</channelNameOverlay>".len();
    extract_xml_tag(&xml[start..end], "name")
}

fn extract_text_overlay_1_value(xml: &str) -> Option<String> {
    let start = xml.find("<TextOverlay")?;
    let rel_end = xml[start..].find("</TextOverlay>")?;
    let end = start + rel_end + "</TextOverlay>".len();
    let text_block = &xml[start..end];
    if text_block.contains("<id>1</id>") || text_block.contains("<id>0</id>") {
        if let Some(enabled) = extract_xml_tag(text_block, "enabled") {
            if enabled == "true" {
                return extract_xml_tag(text_block, "displayText");
            }
        }
    }
    None
}

fn patch_video_overlay_xml(current_ov: &str, escaped_title: &str) -> String {
    let mut updated_ov = current_ov.to_string();
    let is_empty = escaped_title.trim().is_empty();

    // 1. Patch TextOverlay id 1 (Burned directly into video stream)
    if let Some(to_start) = updated_ov.find("<TextOverlay") {
        if let Some(rel_to_end) = updated_ov[to_start..].find("</TextOverlay>") {
            let to_end = to_start + rel_to_end + "</TextOverlay>".len();
            let mut to_block = updated_ov[to_start..to_end].to_string();

            if to_block.contains("<id>1</id>") || to_block.contains("<id>0</id>") {
                if !is_empty {
                    to_block = to_block.replace("<enabled>false</enabled>", "<enabled>true</enabled>");
                } else {
                    to_block = to_block.replace("<enabled>true</enabled>", "<enabled>false</enabled>");
                }

                if to_block.contains("<positionX>0</positionX>") && to_block.contains("<positionY>0</positionY>") {
                    to_block = to_block.replace("<positionX>0</positionX>", "<positionX>64</positionX>");
                    to_block = to_block.replace("<positionY>0</positionY>", "<positionY>64</positionY>");
                }

                if let Some(dt_start) = to_block.find("<displayText>") {
                    if let Some(dt_end) = to_block[dt_start..].find("</displayText>") {
                        let before = &to_block[..dt_start + 13];
                        let after = &to_block[dt_start + dt_end..];
                        to_block = format!("{}{}{}", before, escaped_title, after);
                    }
                } else if let Some(dt_idx) = to_block.find("<displayText/>") {
                    to_block.replace_range(dt_idx..dt_idx + 14, &format!("<displayText>{}</displayText>", escaped_title));
                } else if let Some(dt_idx2) = to_block.find("<displayText />") {
                    to_block.replace_range(dt_idx2..dt_idx2 + 15, &format!("<displayText>{}</displayText>", escaped_title));
                } else if let Some(end_tag) = to_block.rfind("</TextOverlay>") {
                    to_block.insert_str(end_tag, &format!("<displayText>{}</displayText>", escaped_title));
                }

                updated_ov.replace_range(to_start..to_end, &to_block);
            }
        }
    }

    // 2. Patch channelNameOverlay
    if let Some(cno_start) = updated_ov.find("<channelNameOverlay") {
        if let Some(rel_cno_end) = updated_ov[cno_start..].find("</channelNameOverlay>") {
            let cno_end = cno_start + rel_cno_end + "</channelNameOverlay>".len();
            let mut cno_block = updated_ov[cno_start..cno_end].to_string();

            if !is_empty {
                cno_block = cno_block.replace("<enabled>false</enabled>", "<enabled>true</enabled>");
            }

            if let Some(start_name) = cno_block.find("<name>") {
                if let Some(end_name) = cno_block[start_name..].find("</name>") {
                    let before = &cno_block[..start_name + 6];
                    let after = &cno_block[start_name + end_name..];
                    cno_block = format!("{}{}{}", before, escaped_title, after);
                }
            } else if let Some(idx_self_close) = cno_block.find("<name/>") {
                cno_block.replace_range(idx_self_close..idx_self_close + 7, &format!("<name>{}</name>", escaped_title));
            } else if let Some(idx_self_close2) = cno_block.find("<name />") {
                cno_block.replace_range(idx_self_close2..idx_self_close2 + 8, &format!("<name>{}</name>", escaped_title));
            } else if let Some(close_tag) = cno_block.rfind("</channelNameOverlay>") {
                cno_block.insert_str(close_tag, &format!("<name>{}</name>", escaped_title));
            }

            updated_ov.replace_range(cno_start..cno_end, &cno_block);
        }
    } else if let Some(idx_end_ov) = updated_ov.find("</VideoOverlay>") {
        let channel_ov = format!(
            "<channelNameOverlay><enabled>true</enabled><positionX>512</positionX><positionY>64</positionY><name>{}</name></channelNameOverlay>",
            escaped_title
        );
        updated_ov.insert_str(idx_end_ov, &channel_ov);
    }

    updated_ov
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserPermission {
    Admin,
    Operator,
    ViewOnly,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub can_view: bool,
    pub can_change_device_name: bool,
    pub can_change_osd: bool,
    pub can_ptz: bool,
    pub can_audio: bool,
    pub can_snapshot: bool,
    pub can_recording: bool,
    pub user_permission: UserPermission,
    pub protocol_used: String, // "Hikvision ISAPI", "ONVIF", "RTSP Direto"
    pub auth_type: String,     // "Digest", "Basic"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsapiDeviceInfo {
    pub device_name: String,
    pub model: String,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub mac_address: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct DigestState {
    realm: String,
    nonce: String,
    qop: String,
    opaque: String,
    nc: u32,
    is_authenticated: bool,
}

pub struct IsapiClient {
    ip: String,
    port: u16,
    username: String,
    password: String,
    auth_state: Arc<Mutex<DigestState>>,
}

impl IsapiClient {
    pub fn new(ip: &str, port: u16, username: &str, password: &str) -> Self {
        Self {
            ip: ip.to_string(),
            port: if port == 0 { 80 } else { port },
            username: username.to_string(),
            password: password.to_string(),
            auth_state: Arc::new(Mutex::new(DigestState::default())),
        }
    }

    pub async fn http_request(
        &self,
        method: &str,
        uri: &str,
        body: Option<&str>,
    ) -> Result<(u16, String), String> {
        let mut state = self.auth_state.lock().await;

        // Try using existing authenticated session first
        if state.is_authenticated && !state.nonce.is_empty() {
            state.nc += 1;
            let auth_hdr = self.build_digest_header(method, uri, &state);
            let (code, headers, resp_body) = self.raw_send(method, uri, body, Some(&auth_hdr)).await?;
            if code != 401 {
                return Ok((code, resp_body));
            }
            // If 401, nonce became stale, parse new header below
            self.parse_auth_headers(&headers, &mut state);
        }

        // Unauthenticated probe / handshake
        let (code, headers, resp_body) = self.raw_send(method, uri, body, None).await?;
        if code != 401 {
            return Ok((code, resp_body));
        }

        // Parse WWW-Authenticate
        self.parse_auth_headers(&headers, &mut state);

        if state.realm.is_empty() || state.nonce.is_empty() {
            // Try Basic Auth fallback
            let cred = format!("{}:{}", self.username, self.password);
            let basic = format!("Basic {}", BASE64.encode(cred));
            let (code_b, _, resp_b) = self.raw_send(method, uri, body, Some(&basic)).await?;
            return Ok((code_b, resp_b));
        }

        state.nc = 1;
        state.is_authenticated = true;
        let auth_hdr = self.build_digest_header(method, uri, &state);

        let (code2, _, resp_body2) = self.raw_send(method, uri, body, Some(&auth_hdr)).await?;
        Ok((code2, resp_body2))
    }

    fn parse_auth_headers(&self, headers: &str, state: &mut DigestState) {
        for line in headers.lines() {
            let l = line.trim();
            if l.to_lowercase().starts_with("www-authenticate:") {
                let auth_val = l[17..].trim();
                if auth_val.to_lowercase().starts_with("digest") {
                    let params = auth_val[6..].trim();
                    for item in params.split(',') {
                        let part = item.trim();
                        if let Some((k, v)) = part.split_once('=') {
                            let key = k.trim().to_lowercase();
                            let val = v.trim().trim_matches('"').to_string();
                            match key.as_str() {
                                "realm" => state.realm = val,
                                "nonce" => state.nonce = val,
                                "qop" => state.qop = val,
                                "opaque" => state.opaque = val,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    fn build_digest_header(&self, method: &str, uri: &str, state: &DigestState) -> String {
        // HA1 = MD5(username:realm:password)
        let ha1_raw = format!("{}:{}:{}", self.username, state.realm, self.password);
        let ha1 = format!("{:x}", Md5::digest(ha1_raw.as_bytes()));

        // HA2 = MD5(method:uri)
        let ha2_raw = format!("{}:{}", method, uri);
        let ha2 = format!("{:x}", Md5::digest(ha2_raw.as_bytes()));

        let cnonce = Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
        let nc_str = format!("{:08x}", state.nc);

        let response = if state.qop.contains("auth") {
            let resp_raw = format!("{}:{}:{}:{}:auth:{}", ha1, state.nonce, nc_str, cnonce, ha2);
            format!("{:x}", Md5::digest(resp_raw.as_bytes()))
        } else {
            let resp_raw = format!("{}:{}:{}", ha1, state.nonce, ha2);
            format!("{:x}", Md5::digest(resp_raw.as_bytes()))
        };

        let mut auth = format!(
            "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\"",
            self.username, state.realm, state.nonce, uri, response
        );

        if state.qop.contains("auth") {
            auth.push_str(&format!(", qop=auth, nc={}, cnonce=\"{}\"", nc_str, cnonce));
        }
        if !state.opaque.is_empty() {
            auth.push_str(&format!(", opaque=\"{}\"", state.opaque));
        }

        auth
    }

    async fn raw_send(
        &self,
        method: &str,
        uri: &str,
        body: Option<&str>,
        auth_header: Option<&str>,
    ) -> Result<(u16, String, String), String> {
        let addr = format!("{}:{}", self.ip, self.port);
        let mut stream = tokio::time::timeout(
            Duration::from_millis(3000),
            TcpStream::connect(&addr)
        )
        .await
        .map_err(|_| "Tempo limite de conexão esgotado (Timeout ao conectar na porta HTTP)".to_string())?
        .map_err(|e| format!("Falha ao conectar no host {}: {}", addr, e))?;

        let body_bytes = body.unwrap_or("");
        let content_len = body_bytes.as_bytes().len();

        let mut req = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: OnliView-ISAPI/1.0\r\nAccept: */*\r\nConnection: close\r\n",
            method, uri, self.ip
        );

        if let Some(auth) = auth_header {
            req.push_str(&format!("Authorization: {}\r\n", auth));
        }

        if content_len > 0 {
            req.push_str("Content-Type: application/xml; charset=\"UTF-8\"\r\n");
            req.push_str(&format!("Content-Length: {}\r\n", content_len));
        }

        req.push_str("\r\n");
        if content_len > 0 {
            req.push_str(body_bytes);
        }

        stream.write_all(req.as_bytes()).await
            .map_err(|e| format!("Erro ao enviar requisição HTTP: {}", e))?;

        // Read until we know the full body has arrived (via Content-Length) rather than relying
        // solely on connection-close/timeout — some device firmwares trickle out larger XML
        // documents (e.g. VideoOverlay with several overlay slots) slowly enough that a short
        // per-read timeout or an undersized safety cap silently truncates the body mid-document,
        // which then looks like "the expected XML tag isn't there" to callers.
        let mut buf: Vec<u8> = Vec::with_capacity(16384);
        let mut temp = [0u8; 4096];
        let mut headers_end: Option<usize> = None;
        let mut content_length: Option<usize> = None;
        let read_deadline = tokio::time::Instant::now() + Duration::from_millis(5000);

        loop {
            let remaining = read_deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, stream.read(&mut temp)).await {
                Ok(Ok(n)) if n > 0 => {
                    buf.extend_from_slice(&temp[..n]);

                    if headers_end.is_none() {
                        if let Some(pos) = find_bytes(&buf, b"\r\n\r\n") {
                            headers_end = Some(pos + 4);
                            let header_str = String::from_utf8_lossy(&buf[..pos]);
                            content_length = header_str.lines().find_map(|line| {
                                let (key, val) = line.split_once(':')?;
                                if key.trim().eq_ignore_ascii_case("content-length") {
                                    val.trim().parse::<usize>().ok()
                                } else {
                                    None
                                }
                            });
                        }
                    }

                    if let (Some(h_end), Some(clen)) = (headers_end, content_length) {
                        if buf.len() >= h_end + clen {
                            break;
                        }
                    }

                    if buf.len() > 1_048_576 {
                        break;
                    }
                }
                _ => break,
            }
        }

        let full_resp = String::from_utf8_lossy(&buf);
        let (header_part, body_part) = match full_resp.split_once("\r\n\r\n") {
            Some((h, b)) => (h, b),
            None => (full_resp.as_ref(), ""),
        };

        let status_code = header_part.lines().next()
            .and_then(|status_line| {
                let mut parts = status_line.split_whitespace();
                parts.next(); // HTTP/1.1
                parts.next().and_then(|c| c.parse::<u16>().ok())
            })
            .unwrap_or(0);

        Ok((status_code, header_part.to_string(), body_part.to_string()))
    }

    pub async fn get_device_info(&self) -> Result<IsapiDeviceInfo, String> {
        let (code, body) = self.http_request("GET", "/ISAPI/System/deviceInfo", None).await?;
        if code == 401 {
            return Err("Usuário ou senha incorretos".to_string());
        }
        if code != 200 {
            return Err(format!("Dispositivo respondeu com erro HTTP {}", code));
        }

        let device_name = extract_xml_tag(&body, "deviceName").unwrap_or_else(|| "Câmera Hikvision".to_string());
        let model = extract_xml_tag(&body, "model").unwrap_or_else(|| "Dispositivo Hikvision".to_string());
        let serial_number = extract_xml_tag(&body, "serialNumber");
        let firmware_version = extract_xml_tag(&body, "firmwareVersion");
        let mac_address = extract_xml_tag(&body, "macAddress").map(|m| m.to_lowercase().replace('-', ":"));
        let device_type = extract_xml_tag(&body, "deviceType");

        Ok(IsapiDeviceInfo {
            device_name,
            model,
            serial_number,
            firmware_version,
            mac_address,
            device_type,
        })
    }

    pub async fn set_device_name(&self, new_name: &str) -> Result<(), String> {
        let escaped_name = xml_escape(new_name);

        // Some firmwares reject a minimal partial DeviceInfo PUT (missing required fields like
        // <deviceID>, "MessageParametersLack"), so fetch the current full document and patch just
        // <deviceName> in place instead of constructing a new document from scratch.
        let xml = match self.http_request("GET", "/ISAPI/System/deviceInfo", None).await {
            Ok((200, current)) => {
                if let Some(start) = current.find("<deviceName>") {
                    if let Some(end) = current[start..].find("</deviceName>") {
                        let before = &current[..start + "<deviceName>".len()];
                        let after = &current[start + end..];
                        format!("{}{}{}", before, escaped_name, after)
                    } else {
                        current
                    }
                } else {
                    current
                }
            }
            _ => format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><DeviceInfo xmlns="http://www.hikvision.com/ver20/XMLSchema" version="2.0"><deviceName>{}</deviceName></DeviceInfo>"#,
                escaped_name
            ),
        };

        let (code, body) = self.http_request("PUT", "/ISAPI/System/deviceInfo", Some(&xml)).await?;
        if code == 200 || body.contains("<statusValue>200</statusValue>") || body.contains("<statusValue>1</statusValue>") || body.contains("OK") {
            Ok(())
        } else if code == 403 || body.contains("403") {
            Err("Usuário autenticado sem permissão para alterar o nome do dispositivo".to_string())
        } else {
            Err(format!("Falha ao alterar Device Name (HTTP {}): {}", code, body.chars().take(300).collect::<String>()))
        }
    }

    pub async fn get_osd_title(&self, channel_id: u32) -> Result<String, String> {
        // Try 1: Video Overlay /ISAPI/System/Video/inputs/channels/{}/overlays (Check TextOverlay id 1 and channelNameOverlay)
        let uri_ov = format!("/ISAPI/System/Video/inputs/channels/{}/overlays", channel_id);
        if let Ok((200, body_ov)) = self.http_request("GET", &uri_ov, None).await {
            if let Some(text_val) = extract_text_overlay_1_value(&body_ov) {
                if !text_val.trim().is_empty() {
                    return Ok(text_val.trim().to_string());
                }
            }
            if let Some(title) = extract_channel_name_overlay_value(&body_ov) {
                if !title.trim().is_empty() {
                    return Ok(title.trim().to_string());
                }
            }
        }

        // Try 2: Video Input Channel /ISAPI/System/Video/inputs/channels/{} (Used by Video Intercoms / Door Stations)
        let uri_input = format!("/ISAPI/System/Video/inputs/channels/{}", channel_id);
        if let Ok((200, body_input)) = self.http_request("GET", &uri_input, None).await {
            if let Some(title) = extract_xml_tag(&body_input, "name") {
                if !title.trim().is_empty() {
                    return Ok(title.trim().to_string());
                }
            }
        }

        // Try 3: Video Inputs title /ISAPI/System/Video/inputs/channels/{}/title
        let uri_title = format!("/ISAPI/System/Video/inputs/channels/{}/title", channel_id);
        if let Ok((200, body_title)) = self.http_request("GET", &uri_title, None).await {
            if let Some(title) = extract_xml_tag(&body_title, "channelName") {
                if !title.trim().is_empty() {
                    return Ok(title.trim().to_string());
                }
            }
        }

        // Try 4: Streaming Channel 101 or 100 + channel_id
        let ch_id = if channel_id <= 1 { 101 } else { channel_id * 100 + 1 };
        let uri_stream = format!("/ISAPI/Streaming/channels/{}", ch_id);
        if let Ok((200, body)) = self.http_request("GET", &uri_stream, None).await {
            if let Some(title) = extract_xml_tag(&body, "channelName") {
                let t = title.trim();
                if !t.is_empty() && t != "101" && t != "102" && t != "ch1" && t != "ch01" {
                    return Ok(t.to_string());
                }
            }
        }

        Ok(String::new())
    }

    pub async fn set_osd_title(&self, channel_id: u32, new_title: &str) -> Result<(), String> {
        let escaped_title = xml_escape(new_title);
        let mut any_success = false;

        // Method 1: Update VideoOverlay /ISAPI/System/Video/inputs/channels/{}/overlays
        // (Updates both TextOverlay id 1 [direct on-screen DSP] and channelNameOverlay)
        let uri_ov = format!("/ISAPI/System/Video/inputs/channels/{}/overlays", channel_id);
        if let Ok((200, current_ov)) = self.http_request("GET", &uri_ov, None).await {
            let updated_ov = patch_video_overlay_xml(&current_ov, &escaped_title);
            if let Ok((code, body)) = self.http_request("PUT", &uri_ov, Some(&updated_ov)).await {
                if code == 200 || body.contains("<statusValue>1</statusValue>") || body.contains("<statusValue>200</statusValue>") || body.contains("OK") {
                    any_success = true;
                }
            }
        }

        // Method 2: Update VideoInputChannel /ISAPI/System/Video/inputs/channels/{}
        // (Standard for Hikvision Video Intercoms / Door Stations such as DS-KB8112-IM)
        let uri_input = format!("/ISAPI/System/Video/inputs/channels/{}", channel_id);
        if let Ok((200, current_vic)) = self.http_request("GET", &uri_input, None).await {
            let mut updated_vic = current_vic.clone();
            let mut patched = false;

            if let Some(start_name) = updated_vic.find("<name>") {
                if let Some(end_name) = updated_vic[start_name..].find("</name>") {
                    let before = &updated_vic[..start_name + 6];
                    let after = &updated_vic[start_name + end_name..];
                    updated_vic = format!("{}{}{}", before, escaped_title, after);
                    patched = true;
                }
            } else if let Some(idx_self_close) = updated_vic.find("<name/>") {
                updated_vic.replace_range(idx_self_close..idx_self_close + 7, &format!("<name>{}</name>", escaped_title));
                patched = true;
            } else if let Some(idx_self_close2) = updated_vic.find("<name />") {
                updated_vic.replace_range(idx_self_close2..idx_self_close2 + 8, &format!("<name>{}</name>", escaped_title));
                patched = true;
            }

            if patched {
                if let Ok((code, body)) = self.http_request("PUT", &uri_input, Some(&updated_vic)).await {
                    if code == 200 || body.contains("<statusValue>1</statusValue>") || body.contains("<statusValue>200</statusValue>") || body.contains("OK") {
                        any_success = true;
                    }
                }
            }
        }

        // Method 3: Legacy title endpoint /ISAPI/System/Video/inputs/channels/{}/title
        let uri_t = format!("/ISAPI/System/Video/inputs/channels/{}/title", channel_id);
        let xml_title = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><channelTitleOverlay xmlns="http://www.hikvision.com/ver20/XMLSchema" version="2.0"><channelName>{}</channelName></channelTitleOverlay>"#,
            escaped_title
        );
        if let Ok((code_t, body_t)) = self.http_request("PUT", &uri_t, Some(&xml_title)).await {
            if code_t == 200 || body_t.contains("<statusValue>200</statusValue>") || body_t.contains("OK") {
                any_success = true;
            }
        }

        let applied = self.get_osd_title(channel_id).await.unwrap_or_default();
        if applied == new_title || any_success {
            Ok(())
        } else {
            Err(format!(
                "Falha ao gravar OSD no dispositivo. Valor atual: '{}'",
                applied
            ))
        }
    }

    pub async fn discover_streaming_channel_url(&self) -> String {
        // Probe streaming channels via ISAPI
        if let Ok((code, body)) = self.http_request("GET", "/ISAPI/Streaming/channels", None).await {
            if code == 200 {
                if let Some(id_str) = extract_xml_tag(&body, "id") {
                    return format!("/Streaming/Channels/{}", id_str);
                }
            }
        }

        "/Streaming/Channels/101".to_string()
    }

    pub async fn discover_substream_channel_url(&self) -> String {
        if let Ok((code, body)) = self.http_request("GET", "/ISAPI/Streaming/channels", None).await {
            if code == 200 {
                if body.contains("<id>102</id>") {
                    return "/Streaming/Channels/102".to_string();
                }
                if let Some(id_str) = extract_xml_tag(&body, "id") {
                    return format!("/Streaming/Channels/{}", id_str);
                }
            }
        }

        "/Streaming/Channels/102".to_string()
    }

    pub async fn detect_capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities {
            can_view: true,
            can_change_device_name: true,
            can_change_osd: true,
            can_ptz: false,
            can_audio: true,
            can_snapshot: true,
            can_recording: false,
            user_permission: UserPermission::Admin,
            protocol_used: "Hikvision ISAPI".to_string(),
            auth_type: "Digest".to_string(),
        };

        // 1. Check User Level via /ISAPI/Security/users
        if let Ok((code, body)) = self.http_request("GET", "/ISAPI/Security/users", None).await {
            if code == 200 {
                let body_lower = body.to_lowercase();
                if body_lower.contains("administrator") || body_lower.contains("admin") {
                    caps.user_permission = UserPermission::Admin;
                    caps.can_change_device_name = true;
                    caps.can_change_osd = true;
                } else if body_lower.contains("operator") {
                    caps.user_permission = UserPermission::Operator;
                    caps.can_change_device_name = false;
                    caps.can_change_osd = false;
                } else if body_lower.contains("viewer") || body_lower.contains("user") {
                    caps.user_permission = UserPermission::ViewOnly;
                    caps.can_change_device_name = false;
                    caps.can_change_osd = false;
                }
            } else if code == 403 {
                // Non-admin user cannot access /ISAPI/Security/users
                caps.user_permission = UserPermission::ViewOnly;
                caps.can_change_device_name = false;
                caps.can_change_osd = false;
            }
        } else {
            // Fallback: If deviceInfo succeeded with 200, assume admin
            if let Ok(info) = self.get_device_info().await {
                if !info.model.is_empty() {
                    caps.user_permission = UserPermission::Admin;
                }
            }
        }

        caps
    }
}
