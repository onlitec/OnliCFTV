use onliview::camera::crypto::{encrypt_password, decrypt_password};
use onliview::logging::logger::sanitize_credentials;
use onliview::rtsp::client::build_authenticated_rtsp_url;
use onliview::database::Database;
use onliview::camera::model::{CreateCameraInput, BatchCreateCamerasInput, BatchDeviceItem, DeviceType};
use onliview::onvif::discovery::{parse_probe_matches, parse_sadp_matches, infer_device_type};

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
fn test_device_type_classification() {
    // 1. Videoporteiro / Intercom (Lab Device)
    let (t1, l1) = infer_device_type("DS-KB8112-IM", "onvif://www.onvif.org/type/audio_encoder", "PORTAO");
    assert_eq!(t1, DeviceType::Intercom);
    assert_eq!(l1, "Videoporteiro / Comunicação");

    // 2. Câmera IP (Lab Device 172.20.120.53)
    let (t2, l2) = infer_device_type("DS-2CD1301-I", "onvif://www.onvif.org/type/video_encoder", "HIKVISION DS-2CD1301-I");
    assert_eq!(t2, DeviceType::IpCamera);
    assert_eq!(l2, "Câmera IP");

    // 3. NVR / Gravador
    let (t3, _) = infer_device_type("DS-7608NI-K2", "onvif://www.onvif.org/Profile/G", "NVR_SALA_CFTV");
    assert_eq!(t3, DeviceType::Nvr);

    // 4. Tráfego / LPR
    let (t4, _) = infer_device_type("DS-TCG227-AIR", "onvif://www.onvif.org/traffic", "LPR_ENTRADA");
    assert_eq!(t4, DeviceType::TrafficLpr);

    // 5. Câmera PTZ / Speed Dome
    let (t5, _) = infer_device_type("DS-2DE4225IW-DE", "onvif://www.onvif.org/type/ptz", "SPEED_DOME_PATIO");
    assert_eq!(t5, DeviceType::Ptz);
}

#[test]
fn test_sadp_probe_parsing() {
    let sadp_xml = r#"
    <ProbeMatch>
      <DeviceDescription>DS-2CD1301-I</DeviceDescription>
      <IPv4Address>172.20.120.53</IPv4Address>
      <CommandPort>8000</CommandPort>
      <HttpPort>80</HttpPort>
      <MAC>ac-cb-51-7b-0b-54</MAC>
    </ProbeMatch>
    "#;

    let dev = parse_sadp_matches(sadp_xml, "172.20.120.53".to_string()).expect("Should parse SADP match");
    assert_eq!(dev.ip, "172.20.120.53");
    assert_eq!(dev.hardware_model, "DS-2CD1301-I");
    assert_eq!(dev.brand, "Hikvision");
    assert_eq!(dev.device_type, DeviceType::IpCamera);
}

#[test]
fn test_database_crud_and_batch() {
    let db_path = "/tmp/test_onliview_crud_batch_v3.db";
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

    let batch_cam_pass = db.get_camera_decrypted_password(&batch_res[0].id).expect("Decrypt failed");
    assert_eq!(batch_cam_pass, Some("SharedPass99!".to_string()));

    let _ = std::fs::remove_file(db_path);
}
