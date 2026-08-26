use crate::discovery::types::DeviceType;

pub fn infer_device_type(hardware_model: &str, scopes: &str, name: &str) -> (DeviceType, String) {
    let m = hardware_model.to_uppercase();
    let s = scopes.to_lowercase();
    let n = name.to_uppercase();

    // 1. Tráfego / Leitura de Placa (LPR / ANPR) / Radar
    if m.starts_with("DS-TCG") || m.starts_with("DS-TPE") || m.starts_with("IDS-2CD9") 
        || m.starts_with("IDS-T") || m.starts_with("ITC") || m.contains("LPR") 
        || m.contains("ANPR") || s.contains("traffic") || s.contains("anpr") 
        || s.contains("lpr") || s.contains("radar") || s.contains("checkpoint") 
        || n.contains("LPR") || n.contains("TRAFEGO") || n.contains("RADAR") {
        return (DeviceType::TrafficLpr, "Câmera de Tráfego / LPR".to_string());
    }

    // 2. Intercom / Videoporteiro / Comunicação / Controle de Acesso
    if m.starts_with("DS-KB") || m.starts_with("DS-KD") || m.starts_with("DS-KH") 
        || m.starts_with("DS-KV") || m.starts_with("DS-K1") || m.starts_with("DS-K2")
        || m.starts_with("VTO") || m.starts_with("VTH") || m.starts_with("PVIP")
        || m.starts_with("ALLO") || m.contains("DOOR") || m.contains("INTERCOM")
        || s.contains("intercom") || s.contains("door") || s.contains("access_control")
        || n.contains("INTERFONE") || n.contains("VIDEOPORTEIRO") || n.contains("CAMPANHIA")
        || n.contains("PORTAO") {
        return (DeviceType::Intercom, "Videoporteiro / Comunicação".to_string());
    }

    // 3. NVR / DVR / Gravador de Vídeo
    if m.starts_with("DS-76") || m.starts_with("DS-77") || m.starts_with("DS-96") 
        || m.starts_with("DS-71") || m.starts_with("DS-72") || m.starts_with("DS-73") 
        || m.starts_with("DS-81") || m.starts_with("IDS-76") || m.starts_with("IDS-77") 
        || m.starts_with("IDS-96") || m.starts_with("NVD") || m.starts_with("MHDX") 
        || m.starts_with("NVR") || m.starts_with("XVR") || m.starts_with("HCVR")
        || s.contains("profile/g") || s.contains("recording") || s.contains("nvr") || s.contains("dvr") {
        return (DeviceType::Nvr, "NVR / Gravador".to_string());
    }

    // 4. Câmera PTZ / Speed Dome
    if m.starts_with("DS-2DE") || m.starts_with("DS-2DF") || m.starts_with("DS-2DY") 
        || m.starts_with("IDS-2DE") || m.starts_with("SD") || m.contains(" PTZ") 
        || m.contains("SPEED") || s.contains("type/ptz") || s.contains("ptz") || s.contains("speeddome") {
        return (DeviceType::Ptz, "Câmera PTZ / Speed Dome".to_string());
    }

    // 5. Câmera Térmica
    if m.starts_with("DS-2TD") || s.contains("thermal") || s.contains("thermography") || m.contains("THERMAL") {
        return (DeviceType::Thermal, "Câmera Térmica".to_string());
    }

    // 6. Câmera IP Convencional (Bullet, Dome, Turret)
    if m.starts_with("DS-2CD") || m.starts_with("DS-2CV") || m.starts_with("IDS-2CD") 
        || m.starts_with("VIP") || m.starts_with("IPC") || m.starts_with("DH-IPC") 
        || s.contains("video_encoder") || s.contains("streaming") {
        return (DeviceType::IpCamera, "Câmera IP".to_string());
    }

    // 7. Equipamentos de Rede (Roteador / Switch / AP)
    if m.contains("SWITCH") || n.contains("SWITCH") {
        return (DeviceType::Switch, "Switch de Rede".to_string());
    }
    if m.contains("ROUTER") || n.contains("ROUTER") || n.contains("GATEWAY") {
        return (DeviceType::Router, "Roteador".to_string());
    }
    if m.contains("EAP") || m.contains("UAP") || m.contains("UNIFI") || n.contains("ACCESS POINT") {
        return (DeviceType::AccessPoint, "Access Point Wi-Fi".to_string());
    }

    (DeviceType::Other, "Dispositivo CFTV".to_string())
}

pub fn calculate_confidence(
    has_sadp: bool,
    has_onvif: bool,
    has_rtsp: bool,
    has_sdk_port: bool,
    has_model: bool,
    has_oui_match: bool,
    has_http_banner: bool,
) -> u8 {
    let mut score: u8 = 0;

    if has_sadp { score = score.saturating_add(30); }
    if has_onvif { score = score.saturating_add(25); }
    if has_rtsp { score = score.saturating_add(15); }
    if has_sdk_port { score = score.saturating_add(15); }
    if has_model { score = score.saturating_add(20); }
    if has_oui_match { score = score.saturating_add(10); }
    if has_http_banner { score = score.saturating_add(10); }

    // Cap at 99% or 100%
    score.min(99)
}
