use std::collections::HashMap;
use std::fs;

pub struct ArpProvider;

impl ArpProvider {
    pub fn get_arp_table() -> HashMap<String, String> {
        let mut map = HashMap::new();

        if let Ok(content) = fs::read_to_string("/proc/net/arp") {
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let ip = parts[0];
                    let mac = parts[3].to_lowercase().replace('-', ":");
                    if mac != "00:00:00:00:00:00" && !mac.is_empty() {
                        map.insert(ip.to_string(), mac);
                    }
                }
            }
        }

        map
    }

    pub fn lookup_oui_vendor(mac: &str) -> Option<String> {
        let clean = mac.to_lowercase().replace([':', '-'], "");
        if clean.len() < 6 {
            return None;
        }
        let prefix = &clean[..6];

        match prefix {
            // Hikvision OUIs
            "c056e3" | "4419b6" | "c8028f" | "accb51" | "bcba85" | "1868cb" | "ec7443" | "e454e8" | "5803fb" => {
                Some("Hikvision".to_string())
            }
            // Intelbras OUIs
            "f86b40" | "887a02" | "589920" | "e82845" | "34fb58" | "e46f13" => {
                Some("Intelbras".to_string())
            }
            // Dahua OUIs
            "38af29" | "bc325f" | "e0508b" | "ec71db" | "9002a9" | "ac8b98" => {
                Some("Dahua".to_string())
            }
            // Axis OUIs
            "00408c" | "acc8e5" | "b8a44f" => {
                Some("Axis".to_string())
            }
            // TP-Link OUIs
            "50d4f7" | "984827" | "b0be76" | "c006c3" | "d80d17" => {
                Some("TP-Link".to_string())
            }
            // Ubiquiti OUIs
            "24a43c" | "0418d6" | "dc9fdb" | "687251" | "788a20" => {
                Some("Ubiquiti".to_string())
            }
            _ => None,
        }
    }
}
