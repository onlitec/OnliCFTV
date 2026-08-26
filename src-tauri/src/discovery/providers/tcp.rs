use std::time::Duration;
use tokio::net::TcpStream;

#[derive(Debug, Clone, Default)]
pub struct OpenPorts {
    pub rtsp_554: bool,
    pub hikvision_8000: bool,
    pub dahua_37777: bool,
    pub http_80: bool,
    pub http_8080: bool,
    pub https_443: bool,
}

pub struct TcpPortProvider;

impl TcpPortProvider {
    pub async fn check_cctv_ports(ip: &str) -> OpenPorts {
        let mut ports = OpenPorts::default();

        // 1. Fast probe on RTSP 554 and SDK 8000
        let (p554, p8000) = tokio::join!(
            is_port_open(ip, 554, 200),
            is_port_open(ip, 8000, 200)
        );
        ports.rtsp_554 = p554;
        ports.hikvision_8000 = p8000;

        // 2. If neither 554 nor 8000 is open, check Dahua 37777 and HTTP 80/8080
        if !p554 && !p8000 {
            let (p37777, p80, p8080) = tokio::join!(
                is_port_open(ip, 37777, 200),
                is_port_open(ip, 80, 200),
                is_port_open(ip, 8080, 200)
            );
            ports.dahua_37777 = p37777;
            ports.http_80 = p80;
            ports.http_8080 = p8080;
        } else {
            // If RTSP or 8000 is open, also check HTTP 80 quickly
            ports.http_80 = is_port_open(ip, 80, 150).await;
        }

        ports
    }
}

pub async fn is_port_open(ip: &str, port: u16, timeout_ms: u64) -> bool {
    let addr = format!("{}:{}", ip, port);
    tokio::time::timeout(Duration::from_millis(timeout_ms), TcpStream::connect(&addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}
