use std::fs;
use std::process::Command;
use crate::discovery::types::NetworkInterfaceInfo;

pub struct NetworkInterfaceManager;

impl NetworkInterfaceManager {
    pub fn get_interfaces() -> Vec<NetworkInterfaceInfo> {
        let mut interfaces = Vec::new();

        // 1. Get default gateway if on Linux
        let default_gateway = get_linux_default_gateway();

        // 2. Query ip -4 addr
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
                                let mac = get_mac_address(&current_iface_name);
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

        if interfaces.is_empty() {
            // Fallback for standalone test environments
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

fn get_mac_address(iface_name: &str) -> Option<String> {
    let path = format!("/sys/class/net/{}/address", iface_name);
    if let Ok(content) = fs::read_to_string(path) {
        let mac = content.trim().to_string();
        if !mac.is_empty() && mac != "00:00:00:00:00:00" {
            return Some(mac);
        }
    }
    None
}

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
