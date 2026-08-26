use onliview::camera::crypto::{encrypt_password, decrypt_password};
use onliview::logging::logger::sanitize_credentials;
use onliview::rtsp::client::build_authenticated_rtsp_url;
use onliview::database::Database;
use onliview::camera::model::{BatchCreateCamerasInput, BatchDeviceItem, DeviceType};
use onliview::discovery::providers::sadp::parse_sadp_xml;
use onliview::discovery::providers::tcp::OpenPorts;
use onliview::discovery::providers::http::HttpFingerprint;
use onliview::discovery::classifier::{classify_device, ClassificationContext};
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
}

#[test]
fn test_ubuntu_server_classification_not_camera() {
    let ports = OpenPorts {
        http_80: true,
        ssh_22: true,
        postgres_5432: true,
        docker_2375: true,
        ..Default::default()
    };

    let http_fp = HttpFingerprint {
        is_linux_server: true,
        server_header: Some("nginx/1.18.0 (Ubuntu)".to_string()),
        html_title: Some("Welcome to Ubuntu Nginx Server".to_string()),
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: "192.168.1.10",
        mac: Some("00:15:5d:01:23:45"),
        hardware_model: "",
        scopes: "",
        name: "",
        has_sadp: false,
        sadp_model: None,
        has_onvif: false,
        open_ports: &ports,
        http_fp: Some(&http_fp),
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);
    assert_eq!(res.device_type, DeviceType::Server);
    assert!(res.device_type_label.contains("Servidor"));
    assert!(res.contradictions.iter().any(|c| c.contains("Banco de dados")));
}

#[test]
fn test_switch_classification_not_camera() {
    let ports = OpenPorts {
        http_80: true,
        snmp_161: true,
        telnet_23: true,
        ..Default::default()
    };

    let http_fp = HttpFingerprint {
        is_switch: true,
        html_title: Some("TP-Link Easy Smart Switch".to_string()),
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: "192.168.1.254",
        mac: Some("50:d4:f7:11:22:33"), // TP-Link OUI
        hardware_model: "TL-SG108E",
        scopes: "",
        name: "TP-Link Switch",
        has_sadp: false,
        sadp_model: None,
        has_onvif: false,
        open_ports: &ports,
        http_fp: Some(&http_fp),
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);
    assert_eq!(res.device_type, DeviceType::Switch);
    assert_eq!(res.device_type_label, "Switch de Rede");
    assert!(res.confidence_score >= 80);
}

#[test]
fn test_router_gateway_classification() {
    let ports = OpenPorts {
        http_80: true,
        dns_53: true,
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: "172.20.120.1",
        mac: Some("24:a4:3c:00:11:22"), // Ubiquiti OUI
        hardware_model: "",
        scopes: "",
        name: "Gateway",
        has_sadp: false,
        sadp_model: None,
        has_onvif: false,
        open_ports: &ports,
        http_fp: None,
        is_default_gateway: true,
    };

    let res = classify_device(&ctx);
    assert_eq!(res.device_type, DeviceType::Router);
    assert_eq!(res.device_type_label, "Roteador");
}

#[test]
fn test_hikvision_camera_classification_high_confidence() {
    let ports = OpenPorts {
        rtsp_554: true,
        hikvision_8000: true,
        http_80: true,
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: "172.20.120.53",
        mac: Some("ac:cb:51:7b:0b:54"), // Hikvision OUI
        hardware_model: "DS-2CD1301-I",
        scopes: "onvif://www.onvif.org/type/video_encoder onvif://www.onvif.org/Profile/Streaming",
        name: "HIKVISION DS-2CD1301-I",
        has_sadp: true,
        sadp_model: Some("DS-2CD1301-I"),
        has_onvif: true,
        open_ports: &ports,
        http_fp: None,
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);
    assert_eq!(res.device_type, DeviceType::IpCamera);
    assert_eq!(res.device_type_label, "Câmera IP");
    assert_eq!(res.brand, "Hikvision");
    assert!(res.confidence_score >= 95);
    assert_eq!(res.confidence_level, "Confirmado");
}

#[test]
fn test_hikvision_nvr_classification() {
    let ports = OpenPorts {
        rtsp_554: true,
        hikvision_8000: true,
        http_80: true,
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: "172.20.120.100",
        mac: Some("c0:56:e3:11:22:33"),
        hardware_model: "DS-7608NI-K2",
        scopes: "onvif://www.onvif.org/Profile/G",
        name: "NVR_SALA_CFTV",
        has_sadp: true,
        sadp_model: Some("DS-7608NI-K2"),
        has_onvif: true,
        open_ports: &ports,
        http_fp: None,
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);
    assert_eq!(res.device_type, DeviceType::Nvr);
    assert_eq!(res.device_type_label, "NVR / Gravador");
    assert!(res.confidence_score >= 95);
}

#[test]
fn test_unknown_device_classification() {
    let ports = OpenPorts {
        http_8080: true,
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: "192.168.1.88",
        mac: None,
        hardware_model: "",
        scopes: "",
        name: "",
        has_sadp: false,
        sadp_model: None,
        has_onvif: false,
        open_ports: &ports,
        http_fp: None,
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);
    assert_eq!(res.device_type, DeviceType::Other);
    assert_eq!(res.device_type_label, "Dispositivo Desconhecido");
    assert_eq!(res.confidence_level, "Desconhecido");
    assert!(res.confidence_score < 40);
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
fn test_network_interfaces_detection() {
    let ifaces = NetworkInterfaceManager::get_interfaces();
    assert!(!ifaces.is_empty());
}

#[test]
fn test_database_crud_and_batch() {
    let db_path = "/tmp/test_onliview_discovery_v5.db";
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
