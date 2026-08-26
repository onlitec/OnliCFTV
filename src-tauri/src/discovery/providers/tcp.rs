use std::time::Duration;
use tokio::net::TcpStream;

#[derive(Debug, Clone, Default)]
pub struct OpenPorts {
    // CCTV Ports
    pub rtsp_554: bool,
    pub hikvision_8000: bool,
    pub dahua_37777: bool,
    pub http_80: bool,
    pub http_8080: bool,
    pub https_443: bool,

    // Server & Database Ports
    pub ssh_22: bool,
    pub smb_445: bool,
    pub postgres_5432: bool,
    pub mysql_3306: bool,
    pub docker_2375: bool,
    pub rdp_3389: bool,

    // Network & Switch & Router Ports
    pub snmp_161: bool,
    pub telnet_23: bool,
    pub dns_53: bool,
}

pub struct TcpPortProvider;

impl TcpPortProvider {
    pub async fn check_all_ports(ip: &str) -> OpenPorts {
        let mut ports = OpenPorts::default();

        // 1. First probe standard CCTV & Web ports (RTSP 554, SDK 8000, HTTP 80, SSH 22)
        let (p554, p8000, p80, p22) = tokio::join!(
            is_port_open(ip, 554, 180),
            is_port_open(ip, 8000, 180),
            is_port_open(ip, 80, 180),
            is_port_open(ip, 22, 180)
        );

        ports.rtsp_554 = p554;
        ports.hikvision_8000 = p8000;
        ports.http_80 = p80;
        ports.ssh_22 = p22;

        // If no port was open at all in fast check, do a quick pass on others before returning
        let (p37777, p443, p8080, p445, p53, p23) = tokio::join!(
            is_port_open(ip, 37777, 180),
            is_port_open(ip, 443, 180),
            is_port_open(ip, 8080, 180),
            is_port_open(ip, 445, 180),
            is_port_open(ip, 53, 180),
            is_port_open(ip, 23, 180)
        );

        ports.dahua_37777 = p37777;
        ports.https_443 = p443;
        ports.http_8080 = p8080;
        ports.smb_445 = p445;
        ports.dns_53 = p53;
        ports.telnet_23 = p23;

        // If SSH or SMB is open, check database/server ports
        if p22 || p445 || p80 {
            let (p5432, p3306, p2375, p3389, p161) = tokio::join!(
                is_port_open(ip, 5432, 150),
                is_port_open(ip, 3306, 150),
                is_port_open(ip, 2375, 150),
                is_port_open(ip, 3389, 150),
                is_port_open(ip, 161, 150)
            );
            ports.postgres_5432 = p5432;
            ports.mysql_3306 = p3306;
            ports.docker_2375 = p2375;
            ports.rdp_3389 = p3389;
            ports.snmp_161 = p161;
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
