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

        // 1. Tier 1: Fast probe on primary LAN & CCTV ports (75ms timeout is plenty for local LAN)
        let (p554, p8000, p80, p22, p53, p8080) = tokio::join!(
            is_port_open(ip, 554, 75),
            is_port_open(ip, 8000, 75),
            is_port_open(ip, 80, 75),
            is_port_open(ip, 22, 75),
            is_port_open(ip, 53, 75),
            is_port_open(ip, 8080, 75)
        );

        ports.rtsp_554 = p554;
        ports.hikvision_8000 = p8000;
        ports.http_80 = p80;
        ports.ssh_22 = p22;
        ports.dns_53 = p53;
        ports.http_8080 = p8080;

        // If no basic service responded, host is likely offline or inactive; skip deeper probe
        if !p554 && !p8000 && !p80 && !p22 && !p53 && !p8080 {
            return ports;
        }

        // 2. Tier 2: Responsive host -> probe secondary CCTV and infrastructure ports
        let (p37777, p443, p445, p23, p161) = tokio::join!(
            is_port_open(ip, 37777, 75),
            is_port_open(ip, 443, 75),
            is_port_open(ip, 445, 75),
            is_port_open(ip, 23, 75),
            is_port_open(ip, 161, 75)
        );

        ports.dahua_37777 = p37777;
        ports.https_443 = p443;
        ports.smb_445 = p445;
        ports.telnet_23 = p23;
        ports.snmp_161 = p161;

        // 3. Tier 3: If SSH/SMB/HTTP is open, check server databases
        if p22 || p445 || p80 {
            let (p5432, p3306, p2375, p3389) = tokio::join!(
                is_port_open(ip, 5432, 60),
                is_port_open(ip, 3306, 60),
                is_port_open(ip, 2375, 60),
                is_port_open(ip, 3389, 60)
            );
            ports.postgres_5432 = p5432;
            ports.mysql_3306 = p3306;
            ports.docker_2375 = p2375;
            ports.rdp_3389 = p3389;
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
