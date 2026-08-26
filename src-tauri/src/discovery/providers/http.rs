use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Default)]
pub struct HttpFingerprint {
    pub is_hikvision: bool,
    pub is_dahua: bool,
    pub is_intelbras: bool,
    pub title: Option<String>,
    pub server_header: Option<String>,
}

pub struct HttpFingerprintProvider;

impl HttpFingerprintProvider {
    pub async fn fingerprint(ip: &str, port: u16) -> Option<HttpFingerprint> {
        let addr = format!("{}:{}", ip, port);
        let mut stream = tokio::time::timeout(
            Duration::from_millis(300),
            TcpStream::connect(&addr)
        ).await.ok()?.ok()?;

        let request = format!(
            "GET / HTTP/1.1\r\nHost: {}\r\nUser-Agent: OnliView-Discovery/1.0\r\nConnection: close\r\n\r\n",
            ip
        );

        if stream.write_all(request.as_bytes()).await.is_err() {
            return None;
        }

        let mut buf = [0u8; 4096];
        let n = match tokio::time::timeout(Duration::from_millis(300), stream.read(&mut buf)).await {
            Ok(Ok(read_bytes)) if read_bytes > 0 => read_bytes,
            _ => return None,
        };

        let response = String::from_utf8_lossy(&buf[..n]);
        let resp_lower = response.to_lowercase();

        let mut fp = HttpFingerprint::default();

        // 1. Check Server header
        for line in response.lines() {
            if line.to_lowercase().starts_with("server:") {
                fp.server_header = Some(line[7..].trim().to_string());
            }
        }

        // 2. Check known brand fingerprints
        if resp_lower.contains("/doc/index.html") || resp_lower.contains("hikvision") || resp_lower.contains("app-hik") {
            fp.is_hikvision = true;
        } else if resp_lower.contains("dahua") || resp_lower.contains("quick_config") || resp_lower.contains("web/index.html") {
            fp.is_dahua = true;
        } else if resp_lower.contains("intelbras") || resp_lower.contains("sim next") {
            fp.is_intelbras = true;
        }

        Some(fp)
    }
}
