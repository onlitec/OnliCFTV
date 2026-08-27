use crate::discovery::types::DeviceType;
use crate::discovery::providers::tcp::OpenPorts;
use crate::discovery::providers::http::HttpFingerprint;
use crate::discovery::providers::arp::ArpProvider;

#[derive(Debug, Clone)]
pub struct ClassificationContext<'a> {
    pub ip: &'a str,
    pub mac: Option<&'a str>,
    pub hardware_model: &'a str,
    pub scopes: &'a str,
    pub name: &'a str,
    pub has_sadp: bool,
    pub sadp_model: Option<&'a str>,
    pub has_onvif: bool,
    pub has_ssdp: bool,
    pub open_ports: &'a OpenPorts,
    pub http_fp: Option<&'a HttpFingerprint>,
    pub is_default_gateway: bool,
}

#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub device_type: DeviceType,
    pub device_type_label: String,
    pub brand: String,
    pub hardware_model: String,
    pub name: String,
    pub confidence_score: u8,
    pub confidence_level: String, // "Confirmado", "Provável", "Possível", "Desconhecido"
    pub evidences: Vec<String>,
    pub contradictions: Vec<String>,
}

pub fn classify_device(ctx: &ClassificationContext) -> ClassificationResult {
    let m = ctx.hardware_model.to_uppercase();
    let s = ctx.scopes.to_lowercase();
    let n = ctx.name.to_uppercase();

    let mut score_camera: i32 = 0;
    let mut score_nvr: i32 = 0;
    let mut score_dvr: i32 = 0;
    let mut score_intercom: i32 = 0;
    let mut score_ptz: i32 = 0;
    let mut score_traffic_lpr: i32 = 0;
    let mut score_server: i32 = 0;
    let mut score_switch: i32 = 0;
    let mut score_router: i32 = 0;
    let mut score_computer: i32 = 0;

    let mut evidences_cam = Vec::new();
    let mut contradictions_cam = Vec::new();
    let mut evidences_srv = Vec::new();
    let mut evidences_sw = Vec::new();
    let mut evidences_rtr = Vec::new();
    let mut evidences_nvr = Vec::new();
    let mut evidences_dvr = Vec::new();
    let mut evidences_int = Vec::new();
    let mut evidences_cmp = Vec::new();

    let mut detected_brand = "Dispositivo de Rede".to_string();
    let mut detected_model = ctx.hardware_model.to_string();
    if detected_model == "IP Camera" || detected_model == "Hikvision IP Device" {
        detected_model = String::new();
    }

    // 1. MAC OUI Vendor Lookup
    let oui_vendor = ctx.mac.and_then(ArpProvider::lookup_oui_vendor);
    if let Some(ref v) = oui_vendor {
        detected_brand = v.clone();
        if v == "Hikvision" || v == "Dahua" || v == "Intelbras" || v == "Axis" || v == "Uniview" || v == "Hanwha" {
            score_camera += 15;
            score_nvr += 15;
            score_intercom += 15;
            evidences_cam.push(format!("+15 Fabricante CFTV confirmado por MAC OUI ({})", v));
        } else if v == "Cisco" || v == "TP-Link" || v == "Ubiquiti" {
            score_switch += 20;
            score_router += 20;
            evidences_sw.push(format!("+20 Fabricante de Rede confirmado por MAC OUI ({})", v));
        } else if v == "Dell" || v == "HP" || v == "Supermicro" {
            score_server += 25;
            evidences_srv.push(format!("+25 Fabricante de Servidor por MAC OUI ({})", v));
        }
    }

    // 2. SADP Evidence (Hikvision Protocol)
    if ctx.has_sadp {
        detected_brand = "Hikvision".to_string();
        if let Some(sm) = ctx.sadp_model {
            detected_model = sm.to_string();
            let sm_u = sm.to_uppercase();
            if sm_u.starts_with("DS-2CD") || sm_u.starts_with("DS-2CV") || sm_u.starts_with("IDS-2CD") {
                score_camera += 50;
                evidences_cam.push(format!("+50 SADP Hikvision: Modelo de Câmera IP ({})", sm));
            } else if sm_u.starts_with("DS-76") || sm_u.starts_with("DS-77") || sm_u.starts_with("DS-96") || sm_u.starts_with("IDS-76") {
                score_nvr += 60;
                evidences_nvr.push(format!("+60 SADP Hikvision: Modelo de NVR/Gravador ({})", sm));
            } else if sm_u.starts_with("DS-KB") || sm_u.starts_with("DS-KD") || sm_u.starts_with("DS-KH") || sm_u.starts_with("DS-KV") {
                score_intercom += 60;
                evidences_int.push(format!("+60 SADP Hikvision: Modelo de Videoporteiro ({})", sm));
            } else if sm_u.starts_with("DS-3E") {
                score_switch += 60;
                evidences_sw.push(format!("+60 SADP Hikvision: Switch de Rede PoE ({})", sm));
            } else {
                score_camera += 30;
                evidences_cam.push("+30 Resposta de protocolo SADP Hikvision".to_string());
            }
        }
    }

    // 3. ONVIF Evidence
    if ctx.has_onvif {
        if s.contains("video_encoder") || s.contains("streaming") || s.contains("profile/s") || s.contains("profile/t") {
            score_camera += 35;
            evidences_cam.push("+35 ONVIF WS-Discovery: Profile S / Video Encoder detectado".to_string());
        } else if s.contains("profile/g") || s.contains("recording") {
            score_nvr += 45;
            evidences_nvr.push("+45 ONVIF WS-Discovery: Profile G / Gravador detectado".to_string());
        } else {
            score_camera += 20;
            evidences_cam.push("+20 ONVIF WS-Discovery ativo".to_string());
        }
    }

    // 3b. SSDP/UPnP Evidence — a weak, generic signal on its own (many non-camera devices, e.g.
    // smart TVs, printers, and routers, also speak SSDP), so it only nudges the score; it never
    // overrides the "no RTSP/ONVIF/SADP/SDK port" penalty below on its own.
    if ctx.has_ssdp {
        score_camera += 10;
        score_nvr += 8;
        evidences_cam.push("+10 Resposta SSDP/UPnP (sinal fraco, complementar)".to_string());
    }

    // 4. Hardware Model string checks (if present and specific)
    if !m.is_empty() && m != "IP CAMERA" && m != "HIKVISION IP DEVICE" && m != "CFTV DEVICE" {
        if m.starts_with("DS-2CD") || m.starts_with("DS-2CV") || m.starts_with("IDS-2CD")
            || m.starts_with("VIP") || m.starts_with("IPC-") || m.starts_with("DH-IPC") {
            score_camera += 45;
            evidences_cam.push(format!("+45 Modelo específico de Câmera IP ({})", ctx.hardware_model));
        } else if m.starts_with("DS-76") || m.starts_with("DS-77") || m.starts_with("DS-96")
            || m.starts_with("DS-71") || m.starts_with("DS-72") || m.starts_with("DS-73")
            || m.starts_with("DS-81") || m.starts_with("NVD") || m.starts_with("MHDX")
            || m.starts_with("NVR") {
            score_nvr += 55;
            evidences_nvr.push(format!("+55 Modelo específico de NVR/Gravador ({})", ctx.hardware_model));
        } else if m.starts_with("XVR") || m.starts_with("DVR") {
            // Unambiguous vendor naming for hybrid/analog recorders (Dahua XVR, Intelbras/Hikvision DVR
            // lines) — kept distinct from NVR so DeviceType::Dvr is actually reachable.
            score_dvr += 55;
            evidences_dvr.push(format!("+55 Modelo específico de DVR ({})", ctx.hardware_model));
        } else if m.starts_with("DS-KB") || m.starts_with("DS-KD") || m.starts_with("DS-KH")
            || m.starts_with("DS-KV") || m.starts_with("VTO") || m.starts_with("PVIP") || m.starts_with("ALLO") {
            score_intercom += 55;
            evidences_int.push(format!("+55 Modelo específico de Videoporteiro/Intercom ({})", ctx.hardware_model));
        } else if m.starts_with("DS-2DE") || m.starts_with("DS-2DF") || m.starts_with("DS-2DY") || m.starts_with("SD") {
            score_ptz += 55;
        } else if m.starts_with("DS-TCG") || m.starts_with("DS-TPE") || m.starts_with("IDS-2CD9") || m.starts_with("ITC") {
            score_traffic_lpr += 55;
        } else if m.starts_with("DS-3E") || m.starts_with("TL-SG") || m.starts_with("SG") || m.contains("SWITCH") {
            score_switch += 50;
            evidences_sw.push(format!("+50 Modelo de Switch ({})", ctx.hardware_model));
        }
    }

    // 5. Open Ports Evaluation
    let p = ctx.open_ports;

    // CCTV ports
    if p.rtsp_554 {
        score_camera += 25;
        score_nvr += 20;
        score_intercom += 20;
        evidences_cam.push("+25 Porta de Streaming RTSP 554 ativa".to_string());
    }
    if p.rtsp_8554 || p.rtsp_10554 {
        score_camera += 20;
        score_nvr += 15;
        evidences_cam.push("+20 Porta de Streaming RTSP alternativa (8554/10554) ativa".to_string());
    }
    if p.hikvision_8000 {
        score_camera += 35;
        score_nvr += 30;
        score_intercom += 30;
        detected_brand = "Hikvision".to_string();
        evidences_cam.push("+35 Porta de Comando SDK Hikvision 8000 ativa".to_string());
    }
    if p.dahua_37777 {
        score_camera += 35;
        score_nvr += 30;
        detected_brand = "Dahua/Intelbras".to_string();
        evidences_cam.push("+35 Porta de Comando SDK Dahua/Intelbras 37777 ativa".to_string());
    }

    // Server & Database ports (Strong Exclusion for cameras)
    if p.postgres_5432 || p.mysql_3306 {
        score_server += 45;
        score_camera -= 50;
        score_nvr -= 40;
        contradictions_cam.push("-50 Banco de dados SQL ativo (PostgreSQL 5432 / MySQL 3306)".to_string());
        evidences_srv.push("+45 Porta de Banco de Dados ativa".to_string());
    }
    if p.docker_2375 {
        score_server += 40;
        score_camera -= 40;
        contradictions_cam.push("-40 Porta Docker daemon (2375) ativa".to_string());
        evidences_srv.push("+40 Porta Docker 2375 ativa".to_string());
    }
    if p.ssh_22 {
        if !ctx.has_sadp && !ctx.has_onvif && !p.hikvision_8000 && !p.dahua_37777 {
            score_server += 25;
            score_switch += 15;
            score_camera -= 20;
            contradictions_cam.push("-20 Serviço SSH 22 ativo sem perfil CFTV".to_string());
            evidences_srv.push("+25 Porta SSH 22 aberta".to_string());
        }
    }
    if p.smb_445 {
        if p.postgres_5432 || p.ssh_22 || p.docker_2375 {
            score_server += 30;
            evidences_srv.push("+30 Compartilhamento SMB 445 em servidor".to_string());
        } else {
            score_computer += 40;
            score_camera -= 40;
            contradictions_cam.push("-40 Protocolo SMB Windows 445 ativo".to_string());
            evidences_cmp.push("+40 Protocolo SMB Windows 445 ativo".to_string());
        }
    }
    if p.rdp_3389 {
        score_computer += 35;
        score_server += 20;
        score_camera -= 40;
        contradictions_cam.push("-40 Remote Desktop RDP 3389 ativo".to_string());
    }

    // Network & Switch ports
    if p.snmp_161 {
        score_switch += 50;
        score_router += 30;
        score_camera -= 50;
        contradictions_cam.push("-50 Serviço SNMP 161 de gerência de rede ativo".to_string());
        evidences_sw.push("+50 Porta SNMP 161 ativa".to_string());
    }
    if p.telnet_23 {
        score_switch += 20;
        score_router += 20;
        evidences_sw.push("+20 Porta Telnet 23 ativa".to_string());
    }
    if p.dns_53 {
        score_router += 40;
        score_camera -= 40;
        contradictions_cam.push("-40 Servidor DNS 53 ativo".to_string());
        evidences_rtr.push("+40 Servidor DNS 53 ativo".to_string());
    }

    // 6. HTTP Fingerprint
    if let Some(fp) = ctx.http_fp {
        if fp.is_linux_server {
            score_server += 50;
            score_camera -= 50;
            contradictions_cam.push("-50 Banner HTTP indica Servidor Linux/Ubuntu/Nginx/Apache".to_string());
            evidences_srv.push(format!("+50 Banner Web: {}", fp.server_header.as_deref().unwrap_or("Linux Server")));
        }
        if fp.is_switch {
            score_switch += 50;
            score_camera -= 60;
            contradictions_cam.push("-60 Página Web identificada como Switch de Rede".to_string());
            evidences_sw.push(format!("+50 Página Web Switch: {}", fp.html_title.as_deref().unwrap_or("Smart Switch")));
        }
        if fp.is_router {
            score_router += 50;
            score_camera -= 60;
            contradictions_cam.push("-60 Página Web identificada como Roteador".to_string());
            evidences_rtr.push(format!("+50 Página Web Roteador: {}", fp.html_title.as_deref().unwrap_or("Router")));
        }
        if fp.is_hikvision {
            score_camera += 25;
            score_nvr += 20;
            detected_brand = "Hikvision".to_string();
            evidences_cam.push("+25 Interface Web CFTV Hikvision confirmada (/doc/index.html)".to_string());
        }
        if fp.is_dahua {
            score_camera += 25;
            score_nvr += 20;
            detected_brand = "Dahua".to_string();
            evidences_cam.push("+25 Interface Web CFTV Dahua confirmada".to_string());
        }
        if fp.is_intelbras {
            score_camera += 25;
            score_nvr += 20;
            detected_brand = "Intelbras".to_string();
            evidences_cam.push("+25 Interface Web CFTV Intelbras confirmada".to_string());
        }
        if fp.is_axis {
            score_camera += 25;
            detected_brand = "Axis".to_string();
            evidences_cam.push("+25 Interface Web CFTV Axis confirmada".to_string());
        }
        if fp.is_uniview {
            score_camera += 25;
            score_nvr += 20;
            detected_brand = "Uniview".to_string();
            evidences_cam.push("+25 Interface Web CFTV Uniview confirmada".to_string());
        }
        if fp.is_reolink {
            score_camera += 25;
            detected_brand = "Reolink".to_string();
            evidences_cam.push("+25 Interface Web CFTV Reolink confirmada".to_string());
        }
        if fp.is_vivotek {
            score_camera += 25;
            detected_brand = "Vivotek".to_string();
            evidences_cam.push("+25 Interface Web CFTV Vivotek confirmada".to_string());
        }
        if fp.is_bosch {
            score_camera += 25;
            score_nvr += 20;
            detected_brand = "Bosch".to_string();
            evidences_cam.push("+25 Interface Web CFTV Bosch confirmada".to_string());
        }
        if fp.is_hanwha {
            score_camera += 25;
            score_nvr += 20;
            detected_brand = "Hanwha".to_string();
            evidences_cam.push("+25 Interface Web CFTV Hanwha/Wisenet confirmada".to_string());
        }
        if fp.is_tplink {
            score_camera += 25;
            detected_brand = "TP-Link".to_string();
            evidences_cam.push("+25 Interface Web CFTV TP-Link/VIGI confirmada".to_string());
        }
    }

    // 7. Gateway check
    if ctx.is_default_gateway {
        score_router += 45;
        score_camera -= 50;
        contradictions_cam.push("-50 IP é o Gateway Padrão da rede".to_string());
        evidences_rtr.push("+45 IP configurado como Gateway Padrão".to_string());
    }

    // 8. Name heuristic check
    if n.contains("PORTAO") || n.contains("INTERFONE") || n.contains("VIDEOPORTEIRO") {
        score_intercom += 30;
    }
    if n.contains("DVR") || n.contains("XVR") {
        score_dvr += 30;
    } else if n.contains("NVR") || n.contains("GRAVADOR") {
        score_nvr += 30;
    }
    if n.contains("SWITCH") {
        score_switch += 30;
    }
    if n.contains("ROUTER") || n.contains("GATEWAY") {
        score_router += 30;
    }

    // 9. Negative penalty: If device has NO RTSP and NO ONVIF and NO SADP and NO SDK port, heavily penalize Camera
    if !p.rtsp_554 && !ctx.has_onvif && !ctx.has_sadp && !p.hikvision_8000 && !p.dahua_37777 {
        score_camera -= 40;
        contradictions_cam.push("-40 Sem RTSP 554, sem ONVIF, sem SADP e sem porta SDK".to_string());
    }

    // 10. Multi-category Decision Tournament
    let mut scores = vec![
        (DeviceType::Server, score_server, "Servidor", evidences_srv),
        (DeviceType::Switch, score_switch, "Switch de Rede", evidences_sw),
        (DeviceType::Router, score_router, "Roteador", evidences_rtr),
        (DeviceType::Nvr, score_nvr, "NVR / Gravador", evidences_nvr),
        (DeviceType::Dvr, score_dvr, "DVR / Gravador Digital", evidences_dvr),
        (DeviceType::Intercom, score_intercom, "Videoporteiro / Comunicação", evidences_int),
        (DeviceType::Ptz, score_ptz, "Câmera PTZ / Speed Dome", vec![]),
        (DeviceType::TrafficLpr, score_traffic_lpr, "Câmera de Tráfego / LPR", vec![]),
        (DeviceType::Computer, score_computer, "Computador", evidences_cmp),
        (DeviceType::IpCamera, score_camera, "Câmera IP", evidences_cam.clone()),
    ];

    scores.sort_by(|a, b| b.1.cmp(&a.1));
    let (winner_type, winner_score, winner_label, winner_evidences) = scores.remove(0);

    let raw_score: u8 = winner_score.clamp(0, 99) as u8;

    let (final_type, final_label, confidence_level, final_evidences, final_contradictions) = if raw_score < 40 {
        (
            DeviceType::Other,
            "Dispositivo Desconhecido".to_string(),
            "Desconhecido".to_string(),
            vec!["Sinais insuficientes para classificação precisa".to_string()],
            contradictions_cam,
        )
    } else {
        let level = if raw_score >= 90 {
            "Confirmado".to_string()
        } else if raw_score >= 70 {
            "Provável".to_string()
        } else {
            "Possível".to_string()
        };

        let label = if winner_type == DeviceType::Server && ctx.http_fp.as_ref().map(|f| f.is_linux_server).unwrap_or(false) {
            "Servidor Linux / Ubuntu".to_string()
        } else {
            winner_label.to_string()
        };

        (
            winner_type,
            label,
            level,
            if !winner_evidences.is_empty() { winner_evidences } else { evidences_cam },
            contradictions_cam,
        )
    };

    let final_model = if !detected_model.is_empty() {
        detected_model
    } else {
        match final_type {
            DeviceType::Server => "Linux / Windows Server".to_string(),
            DeviceType::Switch => "Network Switch".to_string(),
            DeviceType::Router => "Gateway Router".to_string(),
            DeviceType::Computer => "Workstation PC".to_string(),
            DeviceType::IpCamera => {
                if detected_brand != "Dispositivo de Rede" {
                    format!("{} Câmera IP", detected_brand)
                } else {
                    "Câmera IP".to_string()
                }
            },
            _ => "Dispositivo de Rede".to_string(),
        }
    };

    let final_name = if !ctx.name.is_empty() && !ctx.name.starts_with("IP Camera") && !ctx.name.starts_with("Câmera (") {
        ctx.name.to_string()
    } else {
        format!("{} ({})", final_label, ctx.ip)
    };

    ClassificationResult {
        device_type: final_type,
        device_type_label: final_label,
        brand: detected_brand,
        hardware_model: final_model,
        name: final_name,
        confidence_score: raw_score,
        confidence_level,
        evidences: final_evidences,
        contradictions: final_contradictions,
    }
}
