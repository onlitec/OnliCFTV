use serde::{Serialize, Deserialize};

pub use crate::discovery::types::{DeviceType, DiscoveredDevice, NetworkInterfaceInfo, DiscoveryProgress};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_encrypted: String,
    pub rtsp_port: u16,
    pub rtsp_url: String,
    pub stream_profile: String, // "main" (101), "sub" (102), "custom"
    pub enabled: bool,
    pub device_name: Option<String>,
    pub osd: Option<String>,
    /// "ip_camera" | "nvr" | "dvr" | "intercom". Distingue um gravador de uma
    /// câmera transmissível: só linhas de NVR/DVR são consultadas na Verificação
    /// de Gravações, e só as demais entram no mosaico do Live View.
    pub device_type: String,
    /// Porta HTTP do ISAPI. NVRs frequentemente usam 8000/8080 em vez de 80.
    pub http_port: u16,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCameraInput {
    pub name: String,
    pub host: String,
    pub username: String,
    pub password: Option<String>,
    pub rtsp_port: Option<u16>,
    pub http_port: Option<u16>,
    pub rtsp_url: Option<String>,
    pub stream_profile: Option<String>,
    pub enabled: Option<bool>,
    pub device_name: Option<String>,
    pub osd: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCameraInput {
    pub id: String,
    pub name: Option<String>,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub rtsp_port: Option<u16>,
    pub http_port: Option<u16>,
    pub rtsp_url: Option<String>,
    pub stream_profile: Option<String>,
    pub enabled: Option<bool>,
    pub device_name: Option<String>,
    pub osd: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub codec: Option<String>,
    pub resolution: Option<String>,
    pub fps: Option<f32>,
    pub bitrate: Option<String>,
    pub latency_ms: Option<u64>,
    pub device_name: Option<String>,
    pub osd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeviceItem {
    pub name: String,
    pub host: String,
    pub rtsp_port: u16,
    pub http_port: Option<u16>,
    pub custom_rtsp_url: Option<String>,
    pub device_name: Option<String>,
    pub osd: Option<String>,
    /// Vem da classificação da Descoberta, para que um NVR encontrado na rede já
    /// seja cadastrado como gravador em vez de câmera.
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCreateCamerasInput {
    pub devices: Vec<BatchDeviceItem>,
    pub username: String,
    pub password: Option<String>,
    pub stream_profile: String, // "main" | "sub"
}

pub use crate::camera::isapi::{DeviceCapabilities, UserPermission, IsapiDeviceInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickViewConnectInput {
    pub ip: String,
    pub mac: Option<String>,
    pub rtsp_port: Option<u16>,
    pub http_port: Option<u16>,
    pub username: String,
    pub password: Option<String>,
    pub remember_password: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickViewSetDeviceNameInput {
    pub ip: String,
    pub http_port: Option<u16>,
    pub username: String,
    pub password: Option<String>,
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickViewSetOsdInput {
    pub ip: String,
    pub http_port: Option<u16>,
    pub channel_id: Option<u32>,
    pub username: String,
    pub password: Option<String>,
    pub new_osd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickViewSessionInfo {
    pub ip: String,
    pub rtsp_port: u16,
    pub http_port: u16,
    pub brand: String,
    pub hardware_model: String,
    pub serial_number: Option<String>,
    pub firmware_version: Option<String>,
    pub mac_address: Option<String>,
    pub device_name: String,
    pub osd_text: String,
    pub stream_url: String,
    pub local_mjpeg_url: String,
    pub capabilities: DeviceCapabilities,
    pub metrics: CameraConnectionTestResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedDeviceCredentials {
    pub username: String,
    pub password: String,
}
