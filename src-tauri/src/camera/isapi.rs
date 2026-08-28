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

use crate::discovery::providers::sadp::{extract_xml_blocks, extract_xml_tag};

/// Prazo padrão de leitura de uma resposta ISAPI.
pub const DEFAULT_READ_TIMEOUT_MS: u64 = 5000;

/// Prazo para busca de gravação: o NVR varre o HD antes de responder e passa
/// dos 5s padrão com facilidade em disco grande ou fragmentado.
pub const RECORDING_SEARCH_TIMEOUT_MS: u64 = 15000;

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Um canal de entrada IP configurado num NVR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvrChannel {
    pub id: u32,
    pub name: String,
    /// `None` em canal analógico/local (DVR ou híbrido): não é descompasso de
    /// cadastro, apenas um canal que não vem de câmera IP.
    pub ip_address: Option<String>,
}

/// Uma faixa contínua de vídeo gravado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSegment {
    pub start: String,
    pub end: String,
}

/// Uma página de resultados de `/ISAPI/ContentMgmt/search`.
#[derive(Debug, Clone)]
pub struct RecordingSearchPage {
    pub segments: Vec<RecordingSegment>,
    /// O aparelho sinalizou que há mais resultados além desta página.
    pub has_more: bool,
}

/// Extrai os canais de `/ISAPI/ContentMgmt/InputProxy/channels`.
///
/// Tolerante por natureza: as formas exatas do XML variam entre modelos e
/// firmwares, então um canal sem `id` legível é descartado e qualquer outro
/// campo ausente vira vazio/`None` em vez de derrubar a leitura inteira.
pub fn parse_input_proxy_channels(xml: &str) -> Vec<NvrChannel> {
    let mut channels = Vec::new();

    for block in extract_xml_blocks(xml, "InputProxyChannel") {
        let Some(id) = extract_xml_tag(&block, "id").and_then(|s| s.trim().parse::<u32>().ok())
        else {
            continue;
        };

        // O IP fica dentro do descritor da fonte, não solto no canal.
        let ip_address = extract_xml_blocks(&block, "sourceInputPortDescriptor")
            .first()
            .and_then(|d| extract_xml_tag(d, "ipAddress"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        channels.push(NvrChannel {
            id,
            name: extract_xml_tag(&block, "name").unwrap_or_default(),
            ip_address,
        });
    }

    channels
}

/// Extrai o estado online por canal de `/ISAPI/ContentMgmt/InputProxy/channels/status`.
///
/// O valor é `Option<bool>` de propósito: firmware que não informe `<online>`
/// deve resultar em "desconhecido", nunca em "offline" — dizer que um canal está
/// offline sem base viraria um alarme falso na tela.
pub fn parse_channel_status(xml: &str) -> std::collections::HashMap<u32, Option<bool>> {
    let mut status = std::collections::HashMap::new();

    for block in extract_xml_blocks(xml, "InputProxyChannelStatus") {
        let Some(id) = extract_xml_tag(&block, "id").and_then(|s| s.trim().parse::<u32>().ok())
        else {
            continue;
        };

        let online = extract_xml_tag(&block, "online")
            .map(|v| v.trim().to_ascii_lowercase())
            .and_then(|v| match v.as_str() {
                "true" | "online" | "1" => Some(true),
                "false" | "offline" | "0" => Some(false),
                _ => None,
            });

        status.insert(id, online);
    }

    status
}

/// Extrai uma página de `<CMSearchResult>`.
///
/// Item sem `<timeSpan>` legível é pulado em vez de abortar a página — um
/// registro corrompido não deve apagar os demais segmentos válidos.
pub fn parse_search_result(xml: &str) -> RecordingSearchPage {
    let mut segments = Vec::new();

    for item in extract_xml_blocks(xml, "searchMatchItem") {
        let Some(span) = extract_xml_blocks(&item, "timeSpan").into_iter().next() else {
            continue;
        };
        let (Some(start), Some(end)) = (
            extract_xml_tag(&span, "startTime"),
            extract_xml_tag(&span, "endTime"),
        ) else {
            continue;
        };
        if start.is_empty() || end.is_empty() {
            continue;
        }
        segments.push(RecordingSegment { start, end });
    }

    // "MORE" indica que o aparelho truncou a resposta nesta página.
    let has_more = extract_xml_tag(xml, "responseStatusStrg")
        .map(|s| s.trim().eq_ignore_ascii_case("MORE"))
        .unwrap_or(false);

    RecordingSearchPage { segments, has_more }
}

/// Escolhe com qual token de `qop` responder ao desafio Digest.
///
/// O servidor anuncia uma lista separada por vírgula. Preferimos `auth`, que não
/// exige hash do corpo. Um teste de substring ingênuo (`qop.contains("auth")`)
/// casaria com "auth-int" e responderia no formato errado — 401 permanente com
/// cara de senha incorreta. `None` = servidor sem qop (RFC 2069 legado).
pub fn select_qop(advertised: &str) -> Option<&'static str> {
    let mut has_auth = false;
    let mut has_auth_int = false;

    for token in advertised.split(',') {
        match token.trim().to_ascii_lowercase().as_str() {
            "auth" => has_auth = true,
            "auth-int" => has_auth_int = true,
            _ => {}
        }
    }

    if has_auth {
        Some("auth")
    } else if has_auth_int {
        Some("auth-int")
    } else {
        None
    }
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
        self.http_request_with_timeout(method, uri, body, DEFAULT_READ_TIMEOUT_MS)
            .await
    }

    /// Igual a `http_request`, mas com o prazo de leitura da resposta ajustável.
    /// Busca de gravação em NVR com HD grande passa dos 5s padrão com facilidade.
    pub async fn http_request_with_timeout(
        &self,
        method: &str,
        uri: &str,
        body: Option<&str>,
        read_timeout_ms: u64,
    ) -> Result<(u16, String), String> {
        let mut state = self.auth_state.lock().await;

        // Try using existing authenticated session first
        if state.is_authenticated && !state.nonce.is_empty() {
            state.nc += 1;
            let auth_hdr = self.build_digest_header(method, uri, body, &state);
            let (code, headers, resp_body) = self
                .raw_send(method, uri, body, Some(&auth_hdr), read_timeout_ms)
                .await?;
            if code != 401 {
                return Ok((code, resp_body));
            }
            // If 401, nonce became stale, parse new header below
            self.parse_auth_headers(&headers, &mut state);
        }

        // Unauthenticated probe / handshake
        let (code, headers, resp_body) = self
            .raw_send(method, uri, body, None, read_timeout_ms)
            .await?;
        if code != 401 {
            return Ok((code, resp_body));
        }

        // Parse WWW-Authenticate
        self.parse_auth_headers(&headers, &mut state);

        if state.realm.is_empty() || state.nonce.is_empty() {
            // Try Basic Auth fallback
            let cred = format!("{}:{}", self.username, self.password);
            let basic = format!("Basic {}", BASE64.encode(cred));
            let (code_b, _, resp_b) = self
                .raw_send(method, uri, body, Some(&basic), read_timeout_ms)
                .await?;
            return Ok((code_b, resp_b));
        }

        state.nc = 1;
        state.is_authenticated = true;
        let auth_hdr = self.build_digest_header(method, uri, body, &state);

        let (code2, _, resp_body2) = self
            .raw_send(method, uri, body, Some(&auth_hdr), read_timeout_ms)
            .await?;
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

    fn build_digest_header(
        &self,
        method: &str,
        uri: &str,
        body: Option<&str>,
        state: &DigestState,
    ) -> String {
        // HA1 = MD5(username:realm:password)
        let ha1_raw = format!("{}:{}:{}", self.username, state.realm, self.password);
        let ha1 = format!("{:x}", Md5::digest(ha1_raw.as_bytes()));

        let qop = select_qop(&state.qop);

        // HA2 = MD5(method:uri), exceto em auth-int, que também protege o corpo:
        // MD5(method:uri:MD5(body)). Responder com o formato errado gera 401 eterno.
        let ha2_raw = match qop {
            Some("auth-int") => {
                let body_hash = format!("{:x}", Md5::digest(body.unwrap_or("").as_bytes()));
                format!("{}:{}:{}", method, uri, body_hash)
            }
            _ => format!("{}:{}", method, uri),
        };
        let ha2 = format!("{:x}", Md5::digest(ha2_raw.as_bytes()));

        let cnonce = Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
        let nc_str = format!("{:08x}", state.nc);

        let response = match qop {
            Some(q) => {
                let resp_raw = format!("{}:{}:{}:{}:{}:{}", ha1, state.nonce, nc_str, cnonce, q, ha2);
                format!("{:x}", Md5::digest(resp_raw.as_bytes()))
            }
            None => {
                let resp_raw = format!("{}:{}:{}", ha1, state.nonce, ha2);
                format!("{:x}", Md5::digest(resp_raw.as_bytes()))
            }
        };

        let mut auth = format!(
            "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\"",
            self.username, state.realm, state.nonce, uri, response
        );

        // O qop declarado tem de ser o mesmo usado no cálculo acima — declarar
        // "auth" respondendo a um servidor que só ofereceu "auth-int" é rejeitado.
        if let Some(q) = qop {
            auth.push_str(&format!(", qop={}, nc={}, cnonce=\"{}\"", q, nc_str, cnonce));
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
        read_timeout_ms: u64,
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
        let read_deadline = tokio::time::Instant::now() + Duration::from_millis(read_timeout_ms);

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

    /// Lista os canais de entrada IP configurados no NVR.
    pub async fn get_input_proxy_channels(&self) -> Result<Vec<NvrChannel>, String> {
        let (code, body) = self
            .http_request("GET", "/ISAPI/ContentMgmt/InputProxy/channels", None)
            .await?;

        match code {
            200 => Ok(parse_input_proxy_channels(&body)),
            401 => Err("Usuário ou senha incorretos".to_string()),
            403 => Err("Sem permissão para listar os canais do gravador".to_string()),
            404 | 501 => Err(
                "Este equipamento não expõe a lista de canais IP (ISAPI ContentMgmt/InputProxy). \
                 Pode ser um DVR analógico ou firmware antiga."
                    .to_string(),
            ),
            other => Err(format!("Gravador respondeu com erro HTTP {}", other)),
        }
    }

    /// Estado online de cada canal. Falha aqui não é fatal para a verificação:
    /// o chamador segue sem o estado em vez de abortar o NVR inteiro.
    pub async fn get_input_proxy_channel_status(
        &self,
    ) -> Result<std::collections::HashMap<u32, Option<bool>>, String> {
        let (code, body) = self
            .http_request("GET", "/ISAPI/ContentMgmt/InputProxy/channels/status", None)
            .await?;

        if code == 200 {
            Ok(parse_channel_status(&body))
        } else {
            Err(format!("Status dos canais indisponível (HTTP {})", code))
        }
    }

    /// Busca uma página de gravações de um track no intervalo informado.
    ///
    /// Primeiro POST do projeto — ver `select_qop`, pois é justamente um POST com
    /// corpo que alguns firmwares desafiam com `qop=auth-int`.
    pub async fn search_recordings_page(
        &self,
        track_id: u32,
        start_iso: &str,
        end_iso: &str,
        max_results: u16,
        result_position: u32,
    ) -> Result<RecordingSearchPage, String> {
        let search_id = Uuid::new_v4().to_string();
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<CMSearchDescription>
  <searchID>{}</searchID>
  <trackIDList><trackID>{}</trackID></trackIDList>
  <timeSpanList><timeSpan><startTime>{}</startTime><endTime>{}</endTime></timeSpan></timeSpanList>
  <maxResults>{}</maxResults>
  <searchResultPosition>{}</searchResultPosition>
</CMSearchDescription>"#,
            search_id,
            track_id,
            xml_escape(start_iso),
            xml_escape(end_iso),
            max_results,
            result_position
        );

        let (code, resp) = self
            .http_request_with_timeout(
                "POST",
                "/ISAPI/ContentMgmt/search",
                Some(&body),
                RECORDING_SEARCH_TIMEOUT_MS,
            )
            .await?;

        match code {
            200 => Ok(parse_search_result(&resp)),
            401 => Err("Usuário ou senha incorretos".to_string()),
            403 => Err(
                "Sem permissão para consultar gravações (usuário sem direito de Reprodução Remota)"
                    .to_string(),
            ),
            404 | 501 => Err("Busca de gravações não suportada neste firmware".to_string()),
            other => Err(format!("Gravador respondeu com erro HTTP {}", other)),
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
