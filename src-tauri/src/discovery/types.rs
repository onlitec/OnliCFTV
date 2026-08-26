use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    IpCamera,
    Nvr,
    Dvr,
    Ptz,
    Switch,
    Router,
    Intercom,
    AccessControl,
    Alarm,
    Server,
    Computer,
    AccessPoint,
    Thermal,
    TrafficLpr,
    Other,
}

impl Default for DeviceType {
    fn default() -> Self {
        DeviceType::Other
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub netmask: String,
    pub broadcast: String,
    pub gateway: Option<String>,
    pub mac: Option<String>,
    pub is_up: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: String,
    pub ip: String,
    pub mac: Option<String>,
    pub brand: String,
    pub hardware_model: String,
    pub name: String,
    pub device_type: DeviceType,
    pub device_type_label: String,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub activation_status: Option<String>, // "Ativo", "Aguardando ativação", "Desconhecido"
    pub rtsp_port: u16,
    pub http_port: u16,
    pub sdk_port: u16,
    pub protocols: Vec<String>, // e.g. ["SADP", "ONVIF", "RTSP", "HTTP", "SSH", "SNMP"]
    pub confidence_score: u8,   // 0 a 100%
    pub confidence_level: String, // "Confirmado", "Provável", "Possível", "Desconhecido"
    pub evidences: Vec<String>, // e.g. ["+50 Modelo DS-2CD...", "+20 ONVIF Profile S"]
    pub contradictions: Vec<String>, // e.g. ["-40 Portas de servidor detectadas"]
    pub issues: Vec<String>,    // e.g. ["IP fora da faixa", "Não ativada", "RTSP indisponível"]
    pub xaddrs: String,
    pub is_already_added: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryProgress {
    pub percentage: u8,
    pub phase: String,
    pub devices_found: usize,
    pub active_protocols: Vec<String>,
    pub completed_protocols: Vec<String>,
    pub is_running: bool,
}
