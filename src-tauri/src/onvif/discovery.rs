use std::time::Duration;
use crate::discovery::types::DiscoveredDevice;
use crate::discovery::engine::DiscoveryEngine;

pub struct OnvifDiscovery;

impl OnvifDiscovery {
    pub async fn discover_devices(_timeout: Duration) -> Result<Vec<DiscoveredDevice>, String> {
        Ok(DiscoveryEngine::run_discovery(None, |_| {}).await)
    }
}
