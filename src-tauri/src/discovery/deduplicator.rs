use std::collections::HashMap;
use crate::discovery::types::DiscoveredDevice;

pub struct Deduplicator {
    devices_by_key: HashMap<String, DiscoveredDevice>,
    ip_to_key: HashMap<String, String>,
    mac_to_key: HashMap<String, String>,
    serial_to_key: HashMap<String, String>,
}

impl Deduplicator {
    pub fn new() -> Self {
        Self {
            devices_by_key: HashMap::new(),
            ip_to_key: HashMap::new(),
            mac_to_key: HashMap::new(),
            serial_to_key: HashMap::new(),
        }
    }

    pub fn insert_or_merge(&mut self, dev: DiscoveredDevice) {
        // Find existing key by MAC, Serial, or IP in cascade
        let existing_key = dev.mac.as_ref().and_then(|m| self.mac_to_key.get(m).cloned())
            .or_else(|| dev.serial_number.as_ref().and_then(|s| self.serial_to_key.get(s).cloned()))
            .or_else(|| self.ip_to_key.get(&dev.ip).cloned());

        if let Some(key) = existing_key {
            if let Some(target) = self.devices_by_key.get_mut(&key) {
                // Merge protocols
                for p in dev.protocols {
                    if !target.protocols.contains(&p) {
                        target.protocols.push(p);
                    }
                }

                // Merge MAC & index it
                if target.mac.is_none() && dev.mac.is_some() {
                    target.mac = dev.mac.clone();
                }
                if let Some(ref m) = target.mac {
                    self.mac_to_key.insert(m.clone(), key.clone());
                }

                // Merge Serial & index it
                if target.serial_number.is_none() && dev.serial_number.is_some() {
                    target.serial_number = dev.serial_number.clone();
                }
                if let Some(ref s) = target.serial_number {
                    self.serial_to_key.insert(s.clone(), key.clone());
                }

                // Merge Firmware
                if target.firmware_version.is_none() && dev.firmware_version.is_some() {
                    target.firmware_version = dev.firmware_version.clone();
                }

                // Merge Activation
                if target.activation_status.is_none() && dev.activation_status.is_some() {
                    target.activation_status = dev.activation_status.clone();
                }

                // Merge Ports
                if target.rtsp_port == 0 && dev.rtsp_port > 0 {
                    target.rtsp_port = dev.rtsp_port;
                }
                if target.http_port == 0 && dev.http_port > 0 {
                    target.http_port = dev.http_port;
                }
                if target.sdk_port == 0 && dev.sdk_port > 0 {
                    target.sdk_port = dev.sdk_port;
                }

                // Merge Model / Brand if target has generic info
                if (target.hardware_model == "IP Camera" || target.hardware_model == "Network Switch" || target.hardware_model == "Dispositivo de Rede" || target.hardware_model.is_empty())
                    && !dev.hardware_model.is_empty() && dev.hardware_model != "IP Camera" && dev.hardware_model != "Dispositivo de Rede" {
                    target.hardware_model = dev.hardware_model.clone();
                    target.brand = dev.brand.clone();
                    target.name = dev.name.clone();
                    target.device_type = dev.device_type.clone();
                    target.device_type_label = dev.device_type_label.clone();
                }

                if target.xaddrs.is_empty() && !dev.xaddrs.is_empty() {
                    target.xaddrs = dev.xaddrs.clone();
                }

                // Merge Evidences & Contradictions
                for e in dev.evidences {
                    if !target.evidences.contains(&e) {
                        target.evidences.push(e);
                    }
                }
                for c in dev.contradictions {
                    if !target.contradictions.contains(&c) {
                        target.contradictions.push(c);
                    }
                }

                // Keep highest confidence
                if dev.confidence_score > target.confidence_score {
                    target.confidence_score = dev.confidence_score;
                    target.confidence_level = dev.confidence_level;
                    target.device_type = dev.device_type;
                    target.device_type_label = dev.device_type_label;
                }
            }
        } else {
            // Insert new
            let key = dev.mac.clone().unwrap_or_else(|| dev.ip.clone());
            self.ip_to_key.insert(dev.ip.clone(), key.clone());
            if let Some(ref mac) = dev.mac {
                self.mac_to_key.insert(mac.clone(), key.clone());
            }
            if let Some(ref serial) = dev.serial_number {
                self.serial_to_key.insert(serial.clone(), key.clone());
            }
            self.devices_by_key.insert(key, dev);
        }
    }

    pub fn get_mut(&mut self, ip: &str) -> Option<&mut DiscoveredDevice> {
        let key = self.ip_to_key.get(ip)?.clone();
        self.devices_by_key.get_mut(&key)
    }

    pub fn into_vec(self) -> Vec<DiscoveredDevice> {
        let mut list: Vec<DiscoveredDevice> = self.devices_by_key.into_values().collect();
        list.sort_by(|a, b| a.ip.cmp(&b.ip));
        list
    }
}
