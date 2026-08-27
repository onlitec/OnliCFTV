use std::collections::HashMap;
#[cfg(not(target_os = "windows"))]
use std::fs;

pub struct ArpProvider;

impl ArpProvider {
    #[cfg(not(target_os = "windows"))]
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

    /// Windows has no /proc/net/arp — shell out to the standard `arp -a` table dump instead.
    /// Typical line: "  192.168.1.20          aa-bb-cc-dd-ee-ff     dynamic"
    #[cfg(target_os = "windows")]
    pub fn get_arp_table() -> HashMap<String, String> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut map = HashMap::new();
        let mut cmd = std::process::Command::new("arp");
        cmd.args(["-a"]);
        cmd.creation_flags(CREATE_NO_WINDOW);

        if let Ok(output) = cmd.output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let ip = parts[0];
                    let mac_raw = parts[1];
                    if ip.parse::<std::net::Ipv4Addr>().is_ok() && mac_raw.contains('-') {
                        let mac = mac_raw.to_lowercase().replace('-', ":");
                        if mac != "00:00:00:00:00:00" && mac.len() == 17 {
                            map.insert(ip.to_string(), mac);
                        }
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
            "849459" | "849a40" | "c056e3" | "4419b6" | "c8028f" | "accb51" | "bcba85"
            | "1868cb" | "ec7443" | "e454e8" | "5803fb" | "2857be" | "24698e" | "6014b3"
            | "706a94" | "f84d89" | "b4a382" | "a41437" | "40163b" | "001212" | "d0b5c2"
            | "30f772" | "68db54" | "9c1463" | "085411" | "d43423" | "e8abfa" | "101b54"
            | "38702c" | "9002a9" | "b0f963" | "dc8b28" | "7483c2" | "ac1203"
            | "4cbd8f" | "54c415" | "64db8b" | "94e1ac" | "bcad28" | "c42f90" => {
                Some("Hikvision".to_string())
            }
            // Intelbras OUIs
            "f86b40" | "887a02" | "589920" | "e82845" | "34fb58" | "e46f13" | "001a3f"
            | "002191" | "d824bd" | "305a3a" | "a0f3c1" => {
                Some("Intelbras".to_string())
            }
            // Dahua OUIs
            "38af29" | "bc325f" | "e0508b" | "ec71db" | "ac8b98" | "4ce676" | "14a78b"
            | "b84497" | "282c02" | "a0bdcd" | "e46f14" | "3cef8c" | "4c11bf" => {
                Some("Dahua".to_string())
            }
            // Axis OUIs
            "00408c" | "accc8e" | "b8a44f" => {
                Some("Axis".to_string())
            }
            // Uniview OUIs
            "6cf17e" | "c47905" => {
                Some("Uniview".to_string())
            }
            // Hanwha / Samsung Techwin OUIs
            "e43022" | "000918" => {
                Some("Hanwha".to_string())
            }
            // TP-Link OUIs
            "50d4f7" | "984827" | "b0be76" | "c006c3" | "d80d17" | "e4c32a" | "704f57" => {
                Some("TP-Link".to_string())
            }
            // Ubiquiti OUIs
            "24a43c" | "0418d6" | "dc9fdb" | "687251" | "788a20" | "f09fc2" | "802aa8" => {
                Some("Ubiquiti".to_string())
            }
            // Cisco OUIs
            "00000c" | "000142" | "000143" | "000163" | "000164" | "000196" | "000197" => {
                Some("Cisco".to_string())
            }
            // Dell OUIs
            "001422" | "00188b" | "0019b9" | "001a64" | "00219b" => {
                Some("Dell".to_string())
            }
            // HP OUIs
            "0001e6" | "0002b3" | "000802" | "000bcd" | "000e7f" => {
                Some("HP".to_string())
            }
            _ => None,
        }
    }
}
