use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnvifDeviceDiscovered {
    pub xaddrs: String,
    pub endpoint_reference: String,
    pub scopes: Vec<String>,
    pub hardware: Option<String>,
    pub name: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnvifProfile {
    pub token: String,
    pub name: String,
    pub video_source_token: String,
    pub video_encoder_token: Option<String>,
    pub rtsp_stream_uri: Option<String>,
}

pub struct OnvifService;

impl OnvifService {
    pub async fn discover_devices() -> Result<Vec<OnvifDeviceDiscovered>, String> {
        // Skeleton for WS-Discovery UDP broadcast (239.255.255.250:3702)
        // Ready for future expansion
        Ok(vec![])
    }
}
