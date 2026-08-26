use std::net::Ipv4Addr;
use crate::discovery::types::DiscoveredDevice;

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    pub fn diagnose_device(
        device: &mut DiscoveredDevice,
        selected_iface_ip: &str,
        selected_iface_mask: &str,
    ) {
        let mut issues = Vec::new();

        // 1. Check Subnet Mismatch
        if let (Ok(dev_ip), Ok(if_ip), Ok(mask)) = (
            device.ip.parse::<Ipv4Addr>(),
            selected_iface_ip.parse::<Ipv4Addr>(),
            selected_iface_mask.parse::<Ipv4Addr>(),
        ) {
            let dev_u32 = u32::from(dev_ip);
            let if_u32 = u32::from(if_ip);
            let mask_u32 = u32::from(mask);

            if (dev_u32 & mask_u32) != (if_u32 & mask_u32) {
                issues.push("🟠 IP em outra sub-rede (requer ajuste de IP)".to_string());
            }
        }

        // 2. Check Activation Status
        if let Some(ref status) = device.activation_status {
            if status == "Aguardando ativação" {
                issues.push("🔴 Câmera não ativada (senha inicial pendente)".to_string());
            }
        }

        // 3. Check ONVIF / RTSP status
        if !device.protocols.contains(&"ONVIF".to_string()) && device.protocols.contains(&"SADP".to_string()) {
            issues.push("🟡 ONVIF desativado no firmware".to_string());
        }

        if !device.protocols.contains(&"RTSP".to_string()) && !device.protocols.contains(&"SADP".to_string()) && !device.protocols.contains(&"ONVIF".to_string()) {
            issues.push("🟡 Porta RTSP 554 inacessível".to_string());
        }

        device.issues = issues;
    }
}
