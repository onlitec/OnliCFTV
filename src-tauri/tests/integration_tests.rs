use onliview::camera::crypto::{encrypt_password, decrypt_password};
use onliview::logging::logger::sanitize_credentials;
use onliview::rtsp::client::build_authenticated_rtsp_url;
use onliview::database::Database;
use onliview::camera::model::{CreateCameraInput, UpdateCameraInput};

#[test]
fn test_crypto_roundtrip() {
    let plain = "Admin@12345#Hikvision!";
    let encrypted = encrypt_password(plain).expect("Encryption failed");
    assert_ne!(plain, encrypted);
    let decrypted = decrypt_password(&encrypted).expect("Decryption failed");
    assert_eq!(plain, decrypted);
}

#[test]
fn test_log_sanitizer() {
    let raw = "Connecting to rtsp://admin:SecretPass123@172.20.120.67:554/Streaming/Channels/101 now";
    let sanitized = sanitize_credentials(raw);
    assert!(!sanitized.contains("SecretPass123"));
    assert!(sanitized.contains("admin:***@172.20.120.67:554"));
}

#[test]
fn test_rtsp_url_builder() {
    let url = build_authenticated_rtsp_url("172.20.120.67", 554, "admin", "pass123", "");
    assert_eq!(url, "rtsp://admin:pass123@172.20.120.67:554/Streaming/Channels/101");

    let custom = build_authenticated_rtsp_url("172.20.120.67", 554, "admin", "pass123", "/live/ch1");
    assert_eq!(custom, "rtsp://admin:pass123@172.20.120.67:554/live/ch1");
}

#[test]
fn test_database_crud() {
    let db_path = "/tmp/test_onliview_crud.db";
    let _ = std::fs::remove_file(db_path);

    let db = Database::new(db_path).expect("Failed to open test database");

    // 1. Create
    let cam = db.create_camera(CreateCameraInput {
        name: "Hikvision Portaria".to_string(),
        host: "172.20.120.67".to_string(),
        username: "admin".to_string(),
        password: Some("PassTest123".to_string()),
        rtsp_port: Some(554),
        rtsp_url: None,
        stream_profile: Some("main".to_string()),
        enabled: Some(true),
    }).expect("Create camera failed");

    assert_eq!(cam.name, "Hikvision Portaria");
    assert_eq!(cam.host, "172.20.120.67");

    // 2. Read & Decrypt
    let cameras = db.get_cameras().expect("Get cameras failed");
    assert_eq!(cameras.len(), 1);

    let pass = db.get_camera_decrypted_password(&cam.id).expect("Decrypt failed");
    assert_eq!(pass, Some("PassTest123".to_string()));

    // 3. Update
    let updated = db.update_camera(UpdateCameraInput {
        id: cam.id.clone(),
        name: Some("Hikvision Entrada Atualizada".to_string()),
        host: None,
        username: None,
        password: None,
        rtsp_port: None,
        rtsp_url: None,
        stream_profile: None,
        enabled: Some(false),
    }).expect("Update camera failed");

    assert_eq!(updated.name, "Hikvision Entrada Atualizada");
    assert_eq!(updated.enabled, false);

    // 4. Delete
    db.delete_camera(&cam.id).expect("Delete camera failed");
    let remaining = db.get_cameras().expect("Get cameras failed");
    assert_eq!(remaining.len(), 0);

    let _ = std::fs::remove_file(db_path);
}
