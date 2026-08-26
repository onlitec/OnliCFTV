use onliview::camera::crypto::{encrypt_password, decrypt_password};
use onliview::logging::logger::sanitize_credentials;
use onliview::rtsp::client::build_authenticated_rtsp_url;
use onliview::database::Database;
use onliview::camera::model::{CreateCameraInput, BatchCreateCamerasInput, BatchDeviceItem};
use onliview::onvif::discovery::parse_probe_matches;

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
fn test_onvif_probe_parsing() {
    let sample_xml = r#"
    <soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope" xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
      <soap:Body>
        <wsd:ProbeMatches>
          <wsd:ProbeMatch>
            <wsd:Scopes>onvif://www.onvif.org/type/video_encoder onvif://www.onvif.org/hardware/DS-KB8112-IM onvif://www.onvif.org/name/PORTAO_ENTRADA</wsd:Scopes>
            <wsd:XAddrs>http://172.20.120.67:80/onvif/device_service</wsd:XAddrs>
          </wsd:ProbeMatch>
        </wsd:ProbeMatches>
      </soap:Body>
    </soap:Envelope>
    "#;

    let dev = parse_probe_matches(sample_xml, "172.20.120.67".to_string()).expect("Should parse ONVIF match");
    assert_eq!(dev.ip, "172.20.120.67");
    assert_eq!(dev.hardware_model, "DS-KB8112-IM");
    assert_eq!(dev.name, "PORTAO_ENTRADA");
}

#[test]
fn test_database_crud_and_batch() {
    let db_path = "/tmp/test_onliview_crud_batch.db";
    let _ = std::fs::remove_file(db_path);

    let db = Database::new(db_path).expect("Failed to open test database");

    // 1. Create Individual
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

    // 2. Batch Creation
    let batch_res = db.create_cameras_batch(BatchCreateCamerasInput {
        devices: vec![
            BatchDeviceItem {
                name: "Cam 1 - Estacionamento".to_string(),
                host: "172.20.120.53".to_string(),
                rtsp_port: 554,
                custom_rtsp_url: None,
            },
            BatchDeviceItem {
                name: "Cam 2 - Corredor".to_string(),
                host: "172.20.120.54".to_string(),
                rtsp_port: 554,
                custom_rtsp_url: None,
            },
        ],
        username: "admin".to_string(),
        password: Some("SharedPass99!".to_string()),
        stream_profile: "main".to_string(),
    }).expect("Batch create failed");

    assert_eq!(batch_res.len(), 2);

    let all_cams = db.get_cameras().expect("Get cameras failed");
    assert_eq!(all_cams.len(), 3);

    // Verify decrypted password on batch item
    let batch_cam_pass = db.get_camera_decrypted_password(&batch_res[0].id).expect("Decrypt failed");
    assert_eq!(batch_cam_pass, Some("SharedPass99!".to_string()));

    let _ = std::fs::remove_file(db_path);
}
