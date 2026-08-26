use std::process::Command;

pub struct IcmpProvider;

impl IcmpProvider {
    pub fn ping_quick(ip: &str) -> bool {
        Command::new("ping")
            .args(["-c", "1", "-W", "1", ip])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
