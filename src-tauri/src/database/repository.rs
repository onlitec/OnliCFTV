use rusqlite::{params, Connection, Result};
use chrono::Utc;
use uuid::Uuid;
use std::sync::{Arc, Mutex};
use std::path::Path;

use crate::camera::model::{Camera, CreateCameraInput, UpdateCameraInput, BatchCreateCamerasInput};
use crate::camera::crypto::{encrypt_password, decrypt_password};
use crate::database::schema::CREATE_TABLES_SQL;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        
        // Optimize SQLite performance for CCTV/VMS operations
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
        ")?;
        
        conn.execute_batch(CREATE_TABLES_SQL)?;

        // Lightweight migration for databases created before the `mac` column existed on
        // device_credentials. CREATE TABLE IF NOT EXISTS above is a no-op on an existing table,
        // so add the column here and ignore the "duplicate column" error when it's already present.
        let _ = conn.execute("ALTER TABLE device_credentials ADD COLUMN mac TEXT", []);

        // Lightweight migration for device_name and osd on cameras table
        let _ = conn.execute("ALTER TABLE cameras ADD COLUMN device_name TEXT", []);
        let _ = conn.execute("ALTER TABLE cameras ADD COLUMN osd TEXT", []);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn get_cameras(&self) -> Result<Vec<Camera>> {
        let lock = self.conn.lock().unwrap();
        let mut stmt = lock.prepare(
            "SELECT id, name, host, username, password_encrypted, rtsp_port, rtsp_url, stream_profile, enabled, device_name, osd, created_at, updated_at FROM cameras ORDER BY created_at ASC"
        )?;

        let camera_iter = stmt.query_map([], |row| {
            Ok(Camera {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                username: row.get(3)?,
                password_encrypted: row.get(4)?,
                rtsp_port: row.get(5)?,
                rtsp_url: row.get(6)?,
                stream_profile: row.get(7)?,
                enabled: row.get::<_, i32>(8)? != 0,
                device_name: row.get(9)?,
                osd: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?;

        let mut cameras = Vec::new();
        for cam in camera_iter {
            cameras.push(cam?);
        }
        Ok(cameras)
    }

    pub fn get_camera_by_id(&self, id: &str) -> Result<Option<Camera>> {
        let lock = self.conn.lock().unwrap();
        let mut stmt = lock.prepare(
            "SELECT id, name, host, username, password_encrypted, rtsp_port, rtsp_url, stream_profile, enabled, device_name, osd, created_at, updated_at FROM cameras WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Camera {
                id: row.get(0)?,
                name: row.get(1)?,
                host: row.get(2)?,
                username: row.get(3)?,
                password_encrypted: row.get(4)?,
                rtsp_port: row.get(5)?,
                rtsp_url: row.get(6)?,
                stream_profile: row.get(7)?,
                enabled: row.get::<_, i32>(8)? != 0,
                device_name: row.get(9)?,
                osd: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_camera_decrypted_password(&self, id: &str) -> Result<Option<String>, String> {
        let camera = self.get_camera_by_id(id).map_err(|e| e.to_string())?;
        match camera {
            Some(cam) => decrypt_password(&cam.password_encrypted).map(Some),
            None => Ok(None),
        }
    }

    pub fn create_camera(&self, input: CreateCameraInput) -> Result<Camera, String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let rtsp_port = input.rtsp_port.unwrap_or(554);
        let stream_profile = input.stream_profile.unwrap_or_else(|| "main".to_string());
        
        let rtsp_url = if let Some(url) = input.rtsp_url {
            if url.trim().is_empty() {
                format_default_rtsp_url(&input.host, rtsp_port, &stream_profile)
            } else {
                url
            }
        } else {
            format_default_rtsp_url(&input.host, rtsp_port, &stream_profile)
        };

        let plain_pass = input.password.unwrap_or_default();
        let password_encrypted = encrypt_password(&plain_pass)?;
        let enabled = input.enabled.unwrap_or(true);

        let lock = self.conn.lock().unwrap();
        lock.execute(
            "INSERT INTO cameras (id, name, host, username, password_encrypted, rtsp_port, rtsp_url, stream_profile, enabled, device_name, osd, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                input.name,
                input.host,
                input.username,
                password_encrypted,
                rtsp_port,
                rtsp_url,
                stream_profile,
                if enabled { 1 } else { 0 },
                input.device_name,
                input.osd,
                now,
                now
            ],
        ).map_err(|e| e.to_string())?;

        Ok(Camera {
            id,
            name: input.name,
            host: input.host,
            username: input.username,
            password_encrypted,
            rtsp_port,
            rtsp_url,
            stream_profile,
            enabled,
            device_name: input.device_name,
            osd: input.osd,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn create_cameras_batch(&self, input: BatchCreateCamerasInput) -> Result<Vec<Camera>, String> {
        let now = Utc::now().to_rfc3339();
        let plain_pass = input.password.unwrap_or_default();
        let password_encrypted = encrypt_password(&plain_pass)?;
        let stream_profile = if input.stream_profile.is_empty() { "main".to_string() } else { input.stream_profile };

        let mut lock = self.conn.lock().unwrap();
        let tx = lock.transaction().map_err(|e| e.to_string())?;

        let mut created_cameras = Vec::new();

        for item in input.devices {
            let id = Uuid::new_v4().to_string();
            let rtsp_port = if item.rtsp_port == 0 { 554 } else { item.rtsp_port };
            let rtsp_url = if let Some(url) = item.custom_rtsp_url {
                if url.trim().is_empty() {
                    format_default_rtsp_url(&item.host, rtsp_port, &stream_profile)
                } else {
                    url
                }
            } else {
                format_default_rtsp_url(&item.host, rtsp_port, &stream_profile)
            };

            tx.execute(
                "INSERT INTO cameras (id, name, host, username, password_encrypted, rtsp_port, rtsp_url, stream_profile, enabled, device_name, osd, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    id,
                    item.name,
                    item.host,
                    input.username,
                    password_encrypted,
                    rtsp_port,
                    rtsp_url,
                    stream_profile,
                    1,
                    item.device_name,
                    item.osd,
                    now,
                    now
                ],
            ).map_err(|e| e.to_string())?;

            created_cameras.push(Camera {
                id,
                name: item.name,
                host: item.host,
                username: input.username.clone(),
                password_encrypted: password_encrypted.clone(),
                rtsp_port,
                rtsp_url,
                stream_profile: stream_profile.clone(),
                enabled: true,
                device_name: item.device_name,
                osd: item.osd,
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(created_cameras)
    }

    pub fn update_camera(&self, input: UpdateCameraInput) -> Result<Camera, String> {
        let existing = self.get_camera_by_id(&input.id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Camera not found".to_string())?;

        let now = Utc::now().to_rfc3339();
        let name = input.name.unwrap_or(existing.name);
        let host = input.host.unwrap_or(existing.host);
        let username = input.username.unwrap_or(existing.username);
        let rtsp_port = input.rtsp_port.unwrap_or(existing.rtsp_port);
        let stream_profile = input.stream_profile.unwrap_or(existing.stream_profile);
        let enabled = input.enabled.unwrap_or(existing.enabled);
        let device_name = input.device_name.or(existing.device_name);
        let osd = input.osd.or(existing.osd);

        let password_encrypted = if let Some(pass) = input.password {
            if !pass.is_empty() {
                encrypt_password(&pass)?
            } else {
                existing.password_encrypted
            }
        } else {
            existing.password_encrypted
        };

        let rtsp_url = if let Some(url) = input.rtsp_url {
            if url.trim().is_empty() {
                format_default_rtsp_url(&host, rtsp_port, &stream_profile)
            } else {
                url
            }
        } else {
            existing.rtsp_url
        };

        let lock = self.conn.lock().unwrap();
        lock.execute(
            "UPDATE cameras SET name = ?1, host = ?2, username = ?3, password_encrypted = ?4, rtsp_port = ?5, rtsp_url = ?6, stream_profile = ?7, enabled = ?8, device_name = ?9, osd = ?10, updated_at = ?11 WHERE id = ?12",
            params![
                name,
                host,
                username,
                password_encrypted,
                rtsp_port,
                rtsp_url,
                stream_profile,
                if enabled { 1 } else { 0 },
                device_name,
                osd,
                now,
                input.id
            ],
        ).map_err(|e| e.to_string())?;

        Ok(Camera {
            id: input.id,
            name,
            host,
            username,
            password_encrypted,
            rtsp_port,
            rtsp_url,
            stream_profile,
            enabled,
            device_name,
            osd,
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    pub fn delete_camera(&self, id: &str) -> Result<(), String> {
        let lock = self.conn.lock().unwrap();
        lock.execute("DELETE FROM cameras WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_cameras_batch(&self, ids: &[String]) -> Result<usize, String> {
        let mut lock = self.conn.lock().unwrap();
        let tx = lock.transaction().map_err(|e| e.to_string())?;
        let mut count = 0;
        for id in ids {
            count += tx.execute("DELETE FROM cameras WHERE id = ?1", params![id])
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(count)
    }

    pub fn delete_all_cameras(&self) -> Result<usize, String> {
        let lock = self.conn.lock().unwrap();
        let count = lock.execute("DELETE FROM cameras", []).map_err(|e| e.to_string())?;
        Ok(count)
    }

    /// Caches credentials for a discovered (not necessarily registered) device by IP (and MAC when
    /// known), so the technician isn't asked to retype a password they already entered successfully
    /// once. Called only when the technician opts in via the "remember password" checkbox.
    pub fn save_device_credentials(&self, ip: &str, mac: Option<&str>, username: &str, password: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let password_encrypted = encrypt_password(password)?;
        let mac_norm = mac.map(normalize_mac);

        let lock = self.conn.lock().unwrap();
        lock.execute(
            "INSERT INTO device_credentials (ip, mac, username, password_encrypted, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(ip) DO UPDATE SET mac = excluded.mac, username = excluded.username, password_encrypted = excluded.password_encrypted, updated_at = excluded.updated_at",
            params![ip, mac_norm, username, password_encrypted, now],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Looks up cached credentials by IP first; if not found and a MAC is given, falls back to
    /// matching by MAC — covers the case where the device's IP changed (DHCP) since it was saved.
    pub fn get_device_credentials(&self, ip: &str, mac: Option<&str>) -> Result<Option<(String, String)>, String> {
        let lock = self.conn.lock().unwrap();

        let by_ip = {
            let mut stmt = lock.prepare(
                "SELECT username, password_encrypted FROM device_credentials WHERE ip = ?1"
            ).map_err(|e| e.to_string())?;
            let mut rows = stmt.query(params![ip]).map_err(|e| e.to_string())?;
            match rows.next().map_err(|e| e.to_string())? {
                Some(row) => Some((
                    row.get::<_, String>(0).map_err(|e| e.to_string())?,
                    row.get::<_, String>(1).map_err(|e| e.to_string())?,
                )),
                None => None,
            }
        };

        let found = match (by_ip, mac) {
            (Some(v), _) => Some(v),
            (None, Some(mac_val)) => {
                let mac_norm = normalize_mac(mac_val);
                let mut stmt = lock.prepare(
                    "SELECT username, password_encrypted FROM device_credentials WHERE mac = ?1 LIMIT 1"
                ).map_err(|e| e.to_string())?;
                let mut rows = stmt.query(params![mac_norm]).map_err(|e| e.to_string())?;
                match rows.next().map_err(|e| e.to_string())? {
                    Some(row) => Some((
                        row.get::<_, String>(0).map_err(|e| e.to_string())?,
                        row.get::<_, String>(1).map_err(|e| e.to_string())?,
                    )),
                    None => None,
                }
            }
            (None, None) => None,
        };

        match found {
            Some((username, password_encrypted)) => {
                let password = decrypt_password(&password_encrypted)?;
                Ok(Some((username, password)))
            }
            None => Ok(None),
        }
    }

    /// Removes any cached credentials for the given IP — used when the technician unchecks
    /// "remember password", or explicitly forgets a stale/incorrect saved password.
    pub fn delete_device_credentials(&self, ip: &str) -> Result<(), String> {
        let lock = self.conn.lock().unwrap();
        lock.execute("DELETE FROM device_credentials WHERE ip = ?1", params![ip])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn normalize_mac(mac: &str) -> String {
    mac.trim().to_uppercase()
}

pub fn format_default_rtsp_url(host: &str, port: u16, profile: &str) -> String {
    let channel = match profile {
        "sub" => "102",
        _ => "101",
    };
    format!("rtsp://{}:{}/Streaming/Channels/{}", host, port, channel)
}
