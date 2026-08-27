use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Default)]
pub struct HttpFingerprint {
    pub is_hikvision: bool,
    pub is_dahua: bool,
    pub is_intelbras: bool,
    pub is_axis: bool,
    pub is_uniview: bool,
    pub is_reolink: bool,
    pub is_vivotek: bool,
    pub is_bosch: bool,
    pub is_hanwha: bool,
    pub is_tplink: bool,
    pub is_linux_server: bool,
    pub is_switch: bool,
    pub is_router: bool,
    pub server_header: Option<String>,
    pub html_title: Option<String>,
}

pub struct HttpFingerprintProvider;

impl HttpFingerprintProvider {
    pub async fn fingerprint(ip: &str, port: u16) -> Option<HttpFingerprint> {
        let addr = format!("{}:{}", ip, port);
        let mut stream = tokio::time::timeout(
            Duration::from_millis(150),
            TcpStream::connect(&addr)
        ).await.ok()?.ok()?;

        let request = format!(
            "GET / HTTP/1.1\r\nHost: {}\r\nUser-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64)\r\nAccept: */*\r\nConnection: close\r\n\r\n",
            ip
        );

        if stream.write_all(request.as_bytes()).await.is_err() {
            return None;
        }

        let mut buf = [0u8; 4096];
        let n = match tokio::time::timeout(Duration::from_millis(200), stream.read(&mut buf)).await {
            Ok(Ok(read_bytes)) if read_bytes > 0 => read_bytes,
            _ => return None,
        };

        let response = String::from_utf8_lossy(&buf[..n]);
        let resp_lower = response.to_lowercase();

        let mut fp = HttpFingerprint::default();

        // 1. Extract Server header
        for line in response.lines() {
            let line_trim = line.trim();
            if line_trim.to_lowercase().starts_with("server:") {
                let s_val = line_trim[7..].trim().to_string();
                let s_lower = s_val.to_lowercase();
                if s_lower.contains("ubuntu") || s_lower.contains("debian") || s_lower.contains("centos") 
                    || (s_lower.contains("nginx") && !s_lower.contains("hik")) 
                    || (s_lower.contains("apache") && !s_lower.contains("hik")) {
                    fp.is_linux_server = true;
                } else if s_lower.contains("routeros") || s_lower.contains("openwrt") {
                    fp.is_router = true;
                }
                fp.server_header = Some(s_val);
            }
        }

        // 2. Extract HTML <title> tag
        if let Some(start_t) = resp_lower.find("<title>") {
            if let Some(end_t) = resp_lower[start_t + 7..].find("</title>") {
                let title_val = response[start_t + 7..start_t + 7 + end_t].trim().to_string();
                let t_lower = title_val.to_lowercase();
                
                if t_lower.contains("ubuntu") || t_lower.contains("apache2") || t_lower.contains("debian") || t_lower.contains("portainer") {
                    fp.is_linux_server = true;
                } else if t_lower.contains("switch") || t_lower.contains("smart switch") || t_lower.contains("easy smart") {
                    fp.is_switch = true;
                } else if t_lower.contains("router") || t_lower.contains("openwrt") || t_lower.contains("mikrotik") || t_lower.contains("gateway") {
                    fp.is_router = true;
                }
                
                fp.html_title = Some(title_val);
            }
        }

        // 3. Known CFTV Signatures
        if resp_lower.contains("/doc/index.html") || resp_lower.contains("hikvision") || resp_lower.contains("app-webserver")
            || resp_lower.contains("web version") || resp_lower.contains("web/index.html") {
            fp.is_hikvision = true;
        } else if resp_lower.contains("dahua") || resp_lower.contains("quick_config") {
            fp.is_dahua = true;
        } else if resp_lower.contains("intelbras") || resp_lower.contains("sim next") {
            fp.is_intelbras = true;
        } else if resp_lower.contains("axis") {
            fp.is_axis = true;
        } else if resp_lower.contains("uniview") {
            fp.is_uniview = true;
        } else if resp_lower.contains("reolink") {
            fp.is_reolink = true;
        } else if resp_lower.contains("vivotek") {
            fp.is_vivotek = true;
        } else if resp_lower.contains("bosch") {
            fp.is_bosch = true;
        } else if resp_lower.contains("hanwha") || resp_lower.contains("wisenet") {
            fp.is_hanwha = true;
        } else if resp_lower.contains("tp-link") || resp_lower.contains("vigi") {
            fp.is_tplink = true;
        }

        Some(fp)
    }
}
