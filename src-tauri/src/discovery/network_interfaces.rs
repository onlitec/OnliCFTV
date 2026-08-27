#[cfg(not(target_os = "windows"))]
use std::fs;
use std::process::Command;
use crate::discovery::types::NetworkInterfaceInfo;

pub struct NetworkInterfaceManager;

impl NetworkInterfaceManager {
    pub fn get_interfaces() -> Vec<NetworkInterfaceInfo> {
        #[cfg(target_os = "windows")]
        let mut interfaces = get_windows_interfaces();
        #[cfg(not(target_os = "windows"))]
        let mut interfaces = get_linux_interfaces();

        if interfaces.is_empty() {
            // Last-resort fallback only — if this fires, the active IP sweep will scan a subnet
            // unrelated to the real network, so real interface detection above should never
            // actually fail on a supported OS.
            interfaces.push(NetworkInterfaceInfo {
                id: "eth0".to_string(),
                name: "Ethernet (Padrão)".to_string(),
                ip: "172.20.120.30".to_string(),
                netmask: "255.255.255.0".to_string(),
                broadcast: "172.20.120.255".to_string(),
                gateway: Some("172.20.120.1".to_string()),
                mac: None,
                is_up: true,
                is_default: true,
            });
        }

        // Sort so default/primary ethernet comes first
        interfaces.sort_by(|a, b| b.is_default.cmp(&a.is_default));
        interfaces
    }

    /// Enumerates every usable host address in the subnet described by `ip`/`netmask` (excluding
    /// the network and broadcast addresses), instead of assuming a fixed /24. Falls back to a /24
    /// window around `ip` if the mask fails to parse or would produce a pathologically large range
    /// (guards against a misdetected netmask, e.g. /8, blowing up the scan to millions of addresses).
    pub fn host_ips_in_subnet(ip: &str, netmask: &str, cap: usize) -> Vec<String> {
        let parsed = (ip.parse::<std::net::Ipv4Addr>(), netmask.parse::<std::net::Ipv4Addr>());
        let (ip_v4, mask_v4) = match parsed {
            (Ok(i), Ok(m)) => (i, m),
            _ => return default_24_range(ip),
        };

        let ip_u32 = u32::from(ip_v4);
        let mask_u32 = u32::from(mask_v4);
        if mask_u32 == 0 || mask_u32 == u32::MAX {
            return default_24_range(ip);
        }

        let network = ip_u32 & mask_u32;
        let broadcast = network | !mask_u32;
        let host_count = (broadcast.wrapping_sub(network)).saturating_sub(1) as usize;

        if host_count == 0 || host_count > cap {
            return default_24_range(ip);
        }

        (network + 1..broadcast)
            .map(|addr| std::net::Ipv4Addr::from(addr).to_string())
            .collect()
    }
}

#[cfg(not(target_os = "windows"))]
fn get_linux_interfaces() -> Vec<NetworkInterfaceInfo> {
    let mut interfaces = Vec::new();
    let default_gateway = get_linux_default_gateway();

    if let Ok(output) = Command::new("ip").args(["-4", "addr"]).output() {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut current_iface_name = String::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if !line.starts_with(' ') && !line.starts_with('\t') && line.contains(':') {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    current_iface_name = parts[1].trim().to_string();
                }
            } else if trimmed.starts_with("inet ") && !current_iface_name.is_empty() {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let cidr = parts[1];
                    if let Some(slash_pos) = cidr.find('/') {
                        let ip = cidr[..slash_pos].to_string();
                        let prefix_len: u32 = cidr[slash_pos + 1..].parse().unwrap_or(24);

                        if ip != "127.0.0.1" && !ip.starts_with("127.") {
                            let mut broadcast = String::new();
                            if let Some(brd_idx) = parts.iter().position(|&x| x == "brd") {
                                if brd_idx + 1 < parts.len() {
                                    broadcast = parts[brd_idx + 1].to_string();
                                }
                            }

                            if broadcast.is_empty() {
                                broadcast = calculate_broadcast(&ip, prefix_len);
                            }

                            let netmask = calculate_netmask(prefix_len);
                            let mac = get_linux_mac_address(&current_iface_name);
                            let is_def = default_gateway.is_some() && current_iface_name.starts_with('e');

                            interfaces.push(NetworkInterfaceInfo {
                                id: current_iface_name.clone(),
                                name: get_friendly_interface_name(&current_iface_name),
                                ip,
                                netmask,
                                broadcast,
                                gateway: default_gateway.clone(),
                                mac,
                                is_up: true,
                                is_default: is_def,
                            });
                        }
                    }
                }
            }
        }
    }

    interfaces
}

#[cfg(target_os = "windows")]
fn flatten_json_objects(value: serde_json::Value, out: &mut Vec<serde_json::Value>) {
    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                flatten_json_objects(item, out);
            }
        }
        obj @ serde_json::Value::Object(_) => out.push(obj),
        _ => {}
    }
}

#[cfg(target_os = "windows")]
fn get_windows_interfaces() -> Vec<NetworkInterfaceInfo> {
    use std::os::windows::process::CommandExt;

    // Ask PowerShell for every up IPv4-capable adapter, its addresses/prefix lengths, MAC, and the
    // system's current default gateway, all in one call.
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = r#"
$gw = Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue | Sort-Object -Property RouteMetric | Select-Object -First 1 -ExpandProperty NextHop
$out = @()
Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | ForEach-Object {
    $adapter = $_
    Get-NetIPAddress -InterfaceIndex $adapter.IfIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue | ForEach-Object {
        if ($_.IPAddress -ne '127.0.0.1' -and $_.IPAddress -notlike '169.254.*') {
            $out += [PSCustomObject]@{
                Name = $adapter.InterfaceAlias
                MacAddress = $adapter.MacAddress
                IPAddress = $_.IPAddress
                PrefixLength = $_.PrefixLength
                Gateway = $gw
            }
        }
    }
}
$out | ConvertTo-Json -Compress
"#;

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    // PowerShell's ConvertTo-Json has two well-known footguns: piping a single object yields a
    // bare `{...}` instead of `[{...}]`, and piping an empty array yields literal `null` rather
    // than `[]`. Parse defensively and flatten any shape (object, array, empty, or nested array)
    // into a flat list of interface objects, instead of assuming a specific top-level shape.
    if trimmed.is_empty() {
        return Vec::new();
    }
    let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut items: Vec<serde_json::Value> = Vec::new();
    flatten_json_objects(parsed, &mut items);

    let mut interfaces = Vec::new();
    for item in items {
        let ip = match item.get("IPAddress").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let prefix_len = item.get("PrefixLength").and_then(|v| v.as_u64()).unwrap_or(24) as u32;
        let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("Ethernet").to_string();
        let mac = item.get("MacAddress").and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase().replace('-', ":"));
        let gateway = item.get("Gateway").and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let netmask = calculate_netmask(prefix_len);
        let broadcast = calculate_broadcast(&ip, prefix_len);
        let is_def = gateway.is_some();

        interfaces.push(NetworkInterfaceInfo {
            id: name.clone(),
            name: format!("Ethernet ({})", name),
            ip,
            netmask,
            broadcast,
            gateway,
            mac,
            is_up: true,
            is_default: is_def,
        });
    }

    interfaces
}

#[cfg(not(target_os = "windows"))]
fn get_linux_mac_address(iface_name: &str) -> Option<String> {
    let path = format!("/sys/class/net/{}/address", iface_name);
    if let Ok(content) = fs::read_to_string(path) {
        let mac = content.trim().to_string();
        if !mac.is_empty() && mac != "00:00:00:00:00:00" {
            return Some(mac);
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn get_linux_default_gateway() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/proc/net/route") {
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "00000000" {
                let hex_str = parts[2];
                if hex_str.len() == 8 {
                    if let Ok(val) = u32::from_str_radix(hex_str, 16) {
                        let ip = std::net::Ipv4Addr::from(u32::from_be(val.swap_bytes()));
                        return Some(ip.to_string());
                    }
                }
            }
        }
    }
    None
}

fn calculate_netmask(prefix: u32) -> String {
    let mask_val: u32 = if prefix == 0 {
        0
    } else {
        (!0u32).checked_shl(32 - prefix).unwrap_or(0)
    };
    format!(
        "{}.{}.{}.{}",
        (mask_val >> 24) & 0xFF,
        (mask_val >> 16) & 0xFF,
        (mask_val >> 8) & 0xFF,
        mask_val & 0xFF
    )
}

fn calculate_broadcast(ip: &str, prefix: u32) -> String {
    if let Ok(ipv4) = ip.parse::<std::net::Ipv4Addr>() {
        let ip_u32 = u32::from(ipv4);
        let mask_val = if prefix == 0 { 0 } else { (!0u32) << (32 - prefix) };
        let bcast_u32 = ip_u32 | !mask_val;
        return std::net::Ipv4Addr::from(bcast_u32).to_string();
    }
    format!("{}.255", ip.rsplit_once('.').map(|(p, _)| p).unwrap_or("192.168.1"))
}

fn default_24_range(ip: &str) -> Vec<String> {
    let prefix = ip.rsplit_once('.').map(|(p, _)| p).unwrap_or("172.20.120");
    (1..=254).map(|i| format!("{}.{}", prefix, i)).collect()
}

#[cfg(not(target_os = "windows"))]
fn get_friendly_interface_name(name: &str) -> String {
    if name.starts_with('e') {
        format!("Ethernet ({})", name)
    } else if name.starts_with('w') {
        format!("Wi-Fi ({})", name)
    } else if name.starts_with("tailscale") || name.starts_with("tun") || name.starts_with("vpn") {
        format!("VPN ({})", name)
    } else {
        format!("Interface ({})", name)
    }
}
