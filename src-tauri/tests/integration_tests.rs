use onliview::camera::crypto::{encrypt_password, decrypt_password};
use onliview::logging::logger::sanitize_credentials;
use onliview::rtsp::client::build_authenticated_rtsp_url;
use onliview::database::Database;
use onliview::camera::model::{CreateCameraInput, BatchCreateCamerasInput, BatchDeviceItem, DeviceType};
use onliview::discovery::types::DiscoveredDevice;
use onliview::discovery::providers::sadp::parse_sadp_xml;
use onliview::discovery::providers::onvif::parse_onvif_xml;
use onliview::discovery::classifier::{infer_device_type, calculate_confidence};
use onliview::discovery::diagnostic::DiagnosticEngine;
use onliview::discovery::deduplicator::Deduplicator;
use onliview::discovery::network_interfaces::NetworkInterfaceManager;

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
fn test_sadp_xml_parsing() {
    let sadp_xml = r#"
    <ProbeMatch>
      <DeviceDescription>DS-2CD1301-I</DeviceDescription>
      <DeviceSN>DS-2CD1301-I20200921AAWRE28576815</DeviceSN>
      <SoftwareVersion>V5.4.5build 170123</SoftwareVersion>
      <IPv4Address>172.20.120.53</IPv4Address>
      <CommandPort>8000</CommandPort>
      <HttpPort>80</HttpPort>
      <MAC>ac-cb-51-7b-0b-54</MAC>
      <Activated>true</Activated>
    </ProbeMatch>
    "#;

    let rec = parse_sadp_xml(sadp_xml, "172.20.120.53".to_string()).expect("Should parse SADP record");
    assert_eq!(rec.ip, "172.20.120.53");
    assert_eq!(rec.model, "DS-2CD1301-I");
    assert_eq!(rec.serial, Some("DS-2CD1301-I20200921AAWRE28576815".to_string()));
    assert_eq!(rec.mac, Some("ac:cb:51:7b:0b:54".to_string()));
    assert_eq!(rec.activated, Some(true));
}

#[test]
fn test_confidence_and_diagnostic() {
    let score = calculate_confidence(true, true, true, true, true, true, true);
    assert_eq!(score, 99);

    let mut dev = DiscoveredDevice {
        id: "192.168.1.64".to_string(),
        ip: "192.168.1.64".to_string(),
        mac: Some("ac:cb:51:00:11:22".to_string()),
        brand: "Hikvision".to_string(),
        hardware_model: "DS-2CD2021G1-I".to_string(),
        name: "Hikvision DS-2CD2021G1-I".to_string(),
        device_type: DeviceType::IpCamera,
        device_type_label: "Câmera IP".to_string(),
        serial_number: None,
        firmware_version: None,
        activation_status: Some("Aguardando ativação".to_string()),
        rtsp_port: 554,
        http_port: 80,
        sdk_port: 8000,
        protocols: vec!["SADP".to_string()],
        confidence_score: 85,
        issues: Vec::new(),
        xaddrs: String::new(),
        is_already_added: false,
    };

    DiagnosticEngine::diagnose_device(&mut dev, "172.20.120.30", "255.255.255.0");
    assert!(dev.issues.iter().any(|i| i.contains("outra sub-rede")));
    assert!(dev.issues.iter().any(|i| i.contains("não ativada")));
}

#[test]
fn test_deduplicator_merge() {
    let mut dedup = Deduplicator::new();

    // 1. First discovered via TCP sweep
    dedup.insert_or_merge(DiscoveredDevice {
        id: "172.20.120.53".to_string(),
        ip: "172.20.120.53".to_string(),
        mac: None,
        brand: "Câmera IP".to_string(),
        hardware_model: "IP Camera".to_string(),
        name: "Câmera (172.20.120.53)".to_string(),
        device_type: DeviceType::IpCamera,
        device_type_label: "Câmera IP".to_string(),
        serial_number: None,
        firmware_version: None,
        activation_status: Some("Ativo".to_string()),
        rtsp_port: 554,
        http_port: 80,
        sdk_port: 8000,
        protocols: vec!["TCP".to_string(), "RTSP".to_string()],
        confidence_score: 60,
        issues: Vec::new(),
        xaddrs: String::new(),
        is_already_added: false,
    });

    // 2. Then discovered via SADP
    dedup.insert_or_merge(DiscoveredDevice {
        id: "ac:cb:51:7b:0b:54".to_string(),
        ip: "172.20.120.53".to_string(),
        mac: Some("ac:cb:51:7b:0b:54".to_string()),
        brand: "Hikvision".to_string(),
        hardware_model: "DS-2CD1301-I".to_string(),
        name: "Hikvision DS-2CD1301-I".to_string(),
        device_type: DeviceType::IpCamera,
        device_type_label: "Câmera IP".to_string(),
        serial_number: Some("DS-2CD1301-I12345".to_string()),
        firmware_version: Some("V5.4.5".to_string()),
        activation_status: Some("Ativo".to_string()),
        rtsp_port: 554,
        http_port: 80,
        sdk_port: 8000,
        protocols: vec!["SADP".to_string()],
        confidence_score: 95,
        issues: Vec::new(),
        xaddrs: String::new(),
        is_already_added: false,
    });

    let merged = dedup.into_vec();
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].hardware_model, "DS-2CD1301-I");
    assert_eq!(merged[0].mac, Some("ac:cb:51:7b:0b:54".to_string()));
    assert!(merged[0].protocols.contains(&"RTSP".to_string()));
    assert!(merged[0].protocols.contains(&"SADP".to_string()));
}

#[test]
fn test_network_interfaces_detection() {
    let ifaces = NetworkInterfaceManager::get_interfaces();
    assert!(!ifaces.is_empty());
    let default_iface = ifaces.iter().find(|i| i.is_default);
    assert!(default_iface.is_some() || !ifaces.is_empty());
}

#[test]
fn test_database_crud_and_batch() {
    let db_path = "/tmp/test_onliview_discovery_v4.db";
    let _ = std::fs::remove_file(db_path);

    let db = Database::new(db_path).expect("Failed to open test database");

    let batch_res = db.create_cameras_batch(BatchCreateCamerasInput {
        devices: vec![
            BatchDeviceItem {
                name: "Hikvision Portaria".to_string(),
                host: "172.20.120.67".to_string(),
                rtsp_port: 554,
                custom_rtsp_url: None,
            },
            BatchDeviceItem {
                name: "Hikvision Dome".to_string(),
                host: "172.20.120.53".to_string(),
                rtsp_port: 554,
                custom_rtsp_url: None,
            },
        ],
        username: "admin".to_string(),
        password: Some("SharedPass99!".to_string()),
        stream_profile: "main".to_string(),
    }).expect("Batch create failed");

    assert_eq!(batch_res.len(), 2);
    let _ = std::fs::remove_file(db_path);
}
