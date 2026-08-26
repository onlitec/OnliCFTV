use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    pub video_server_port: u16,
    pub database_path: PathBuf,
    pub auto_reconnect_interval_secs: u64,
    pub default_grid_layout: usize, // 1, 4, 9
}

impl Default for AppConfig {
    fn default() -> Self {
        let db_dir = dirs_or_local();
        std::fs::create_dir_all(&db_dir).unwrap_or_default();
        let database_path = db_dir.join("onliview.db");

        Self {
            app_name: "OnliView".to_string(),
            video_server_port: 18554,
            database_path,
            auto_reconnect_interval_secs: 5,
            default_grid_layout: 4,
        }
    }
}

fn dirs_or_local() -> PathBuf {
    if let Some(config_dir) = dirs_config_dir() {
        config_dir.join("onliview")
    } else {
        PathBuf::from("database")
    }
}

fn dirs_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
    }
}
