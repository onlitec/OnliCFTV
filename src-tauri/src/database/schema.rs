pub const CREATE_TABLES_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS cameras (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    username TEXT NOT NULL,
    password_encrypted TEXT NOT NULL,
    rtsp_port INTEGER NOT NULL DEFAULT 554,
    rtsp_url TEXT NOT NULL,
    stream_profile TEXT NOT NULL DEFAULT 'main',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cameras_enabled ON cameras(enabled);

CREATE TABLE IF NOT EXISTS device_credentials (
    ip TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    password_encrypted TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;
