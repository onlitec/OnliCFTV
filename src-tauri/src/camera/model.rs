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
    pub rtsp_url: Option<String>,
    pub stream_profile: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCameraInput {
    pub id: String,
    pub name: Option<String>,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub rtsp_port: Option<u16>,
    pub rtsp_url: Option<String>,
    pub stream_profile: Option<String>,
    pub enabled: Option<bool>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeviceItem {
    pub name: String,
    pub host: String,
    pub rtsp_port: u16,
    pub custom_rtsp_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCreateCamerasInput {
    pub devices: Vec<BatchDeviceItem>,
    pub username: String,
    pub password: Option<String>,
    pub stream_profile: String, // "main" | "sub"
}
