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
        has_ssdp: false,
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
        has_ssdp: false,
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
        has_ssdp: false,
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
        has_ssdp: false,
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
        has_ssdp: false,
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
        has_ssdp: false,
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
                http_port: Some(80),
                custom_rtsp_url: None,
                device_name: Some("CAM-PORTARIA-01".to_string()),
                osd: Some("PORTARIA".to_string()),
                device_type: None, // ausente deve cair no padrao "ip_camera"
            },
            BatchDeviceItem {
                name: "NVR Sala Tecnica".to_string(),
                host: "172.20.120.100".to_string(),
                rtsp_port: 554,
                http_port: Some(8000), // NVR fora da porta 80, caso que motivou persistir http_port
                custom_rtsp_url: None,
                device_name: None,
                osd: None,
                device_type: Some("nvr".to_string()),
            },
        ],
        username: "admin".to_string(),
        password: Some("SharedPass99!".to_string()),
        stream_profile: "main".to_string(),
    }).expect("Batch create failed");

    assert_eq!(batch_res.len(), 2);
    assert_eq!(batch_res[0].device_name, Some("CAM-PORTARIA-01".to_string()));
    assert_eq!(batch_res[0].osd, Some("PORTARIA".to_string()));
    assert_eq!(batch_res[1].device_name, None);

    // device_type e http_port precisam sobreviver ao round-trip no banco: antes
    // http_port era aceito na entrada e descartado, prendendo todo dispositivo
    // salvo a porta 80 e inviabilizando consultar um NVR em 8000.
    assert_eq!(batch_res[0].device_type, "ip_camera");
    assert_eq!(batch_res[0].http_port, 80);
    assert_eq!(batch_res[1].device_type, "nvr");
    assert_eq!(batch_res[1].http_port, 8000);

    let reloaded = db.get_cameras().expect("Failed to reload cameras");
    let nvr = reloaded
        .iter()
        .find(|c| c.host == "172.20.120.100")
        .expect("NVR nao encontrado apos releitura");
    assert_eq!(nvr.device_type, "nvr");
    assert_eq!(nvr.http_port, 8000);

    let _ = std::fs::remove_file(db_path);
}

#[test]
fn test_camera_172_20_120_44_without_sadp_onvif() {
    let ports = OpenPorts {
        rtsp_554: true,
        hikvision_8000: true,
        http_80: true,
        ..Default::default()
    };

    let http_fp = HttpFingerprint {
        is_hikvision: true,
        server_header: Some("webserver".to_string()),
        ..Default::default()
    };

    let ctx = ClassificationContext {
        ip: "172.20.120.44",
        mac: Some("84:94:59:ef:82:00"), // Hikvision OUI 84:94:59
        hardware_model: "",
        scopes: "",
        name: "",
        has_sadp: false,
        sadp_model: None,
        has_onvif: false,
        has_ssdp: false,
        open_ports: &ports,
        http_fp: Some(&http_fp),
        is_default_gateway: false,
    };

    let res = classify_device(&ctx);
    assert_eq!(res.device_type, DeviceType::IpCamera);
    assert_eq!(res.device_type_label, "Câmera IP");
    assert_eq!(res.brand, "Hikvision");
    assert!(res.confidence_score >= 80);
}

#[test]
fn test_isapi_xml_parsing_device_info() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <DeviceInfo xmlns="http://www.hikvision.com/ver20/XMLSchema" version="2.0">
      <deviceName>PORTARIA-PRINCIPAL</deviceName>
      <deviceID>88</deviceID>
      <model>DS-KB8112-IM</model>
      <serialNumber>DS-KB8112-IM20180905AAWR12345678</serialNumber>
      <macAddress>c0:56:e3:11:22:33</macAddress>
      <firmwareVersion>V1.4.71build 170714</firmwareVersion>
      <deviceType>IPDoorStation</deviceType>
    </DeviceInfo>"#;

    assert_eq!(onliview::discovery::providers::sadp::extract_xml_tag(xml, "deviceName"), Some("PORTARIA-PRINCIPAL".to_string()));
    assert_eq!(onliview::discovery::providers::sadp::extract_xml_tag(xml, "model"), Some("DS-KB8112-IM".to_string()));
    assert_eq!(onliview::discovery::providers::sadp::extract_xml_tag(xml, "firmwareVersion"), Some("V1.4.71build 170714".to_string()));
    assert_eq!(onliview::discovery::providers::sadp::extract_xml_tag(xml, "deviceType"), Some("IPDoorStation".to_string()));
}

#[test]
fn test_isapi_xml_parsing_osd_title() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <channelTitleOverlay xmlns="http://www.hikvision.com/ver20/XMLSchema" version="2.0">
      <channelName>PORTAO DE ENTRADA</channelName>
    </channelTitleOverlay>"#;

    assert_eq!(onliview::discovery::providers::sadp::extract_xml_tag(xml, "channelName"), Some("PORTAO DE ENTRADA".to_string()));
}

#[test]
fn test_extract_xml_blocks_repeated_siblings() {
    // Forma da resposta de /ISAPI/ContentMgmt/InputProxy/channels: o invólucro
    // <InputProxyChannelList> tem o nome do item como prefixo, que é exatamente
    // onde um casamento por substring ingênuo erraria.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <InputProxyChannelList>
      <InputProxyChannel>
        <id>1</id>
        <name>ENTRADA</name>
        <sourceInputPortDescriptor>
          <ipAddress>10.10.1.10</ipAddress>
          <managePortNo>8000</managePortNo>
        </sourceInputPortDescriptor>
      </InputProxyChannel>
      <InputProxyChannel>
        <id>2</id>
        <name>GARAGEM</name>
        <sourceInputPortDescriptor>
          <ipAddress>10.10.1.11</ipAddress>
        </sourceInputPortDescriptor>
      </InputProxyChannel>
    </InputProxyChannelList>"#;

    let blocks = onliview::discovery::providers::sadp::extract_xml_blocks(xml, "InputProxyChannel");
    assert_eq!(blocks.len(), 2, "deve encontrar 2 canais, nao o involucro ...List");

    let ids: Vec<Option<String>> = blocks
        .iter()
        .map(|b| onliview::discovery::providers::sadp::extract_xml_tag(b, "id"))
        .collect();
    assert_eq!(ids, vec![Some("1".to_string()), Some("2".to_string())]);

    // O IP fica aninhado, entao precisa recortar o descritor antes de extrair.
    let ip = onliview::discovery::providers::sadp::extract_xml_blocks(&blocks[1], "sourceInputPortDescriptor")
        .first()
        .and_then(|d| onliview::discovery::providers::sadp::extract_xml_tag(d, "ipAddress"));
    assert_eq!(ip, Some("10.10.1.11".to_string()));
}

#[test]
fn test_extract_xml_blocks_edge_cases() {
    use onliview::discovery::providers::sadp::extract_xml_blocks;

    // Sem ocorrencia alguma.
    assert!(extract_xml_blocks("<Outro><a>1</a></Outro>", "searchMatchItem").is_empty());

    // Auto-fechado rende bloco vazio e nao trava a varredura do proximo.
    let selfclosing = r#"<list><item/><item>ok</item><item /></list>"#;
    assert_eq!(extract_xml_blocks(selfclosing, "item"), vec!["", "ok", ""]);

    // Bloco sem fechamento encerra a varredura em vez de panicar ou gerar lixo.
    let truncated = r#"<list><item>completo</item><item>cortado ao meio"#;
    assert_eq!(extract_xml_blocks(truncated, "item"), vec!["completo"]);

    // Atributos no elemento nao atrapalham o casamento.
    let with_attrs = r#"<list><item version="2.0">A</item><item>B</item></list>"#;
    assert_eq!(extract_xml_blocks(with_attrs, "item"), vec!["A", "B"]);
}

#[test]
fn test_select_qop_prefers_auth_and_detects_auth_int() {
    use onliview::camera::isapi::select_qop;

    // Caso comum.
    assert_eq!(select_qop("auth"), Some("auth"));

    // Servidor oferecendo os dois: preferir auth, que dispensa hash do corpo.
    assert_eq!(select_qop("auth,auth-int"), Some("auth"));
    assert_eq!(select_qop("auth-int, auth"), Some("auth"));

    // So auth-int: a regressao que um `contains("auth")` ingenuo causaria era
    // responder no formato de "auth", gerando 401 eterno com cara de senha errada.
    assert_eq!(select_qop("auth-int"), Some("auth-int"));
    assert_eq!(select_qop(" AUTH-INT "), Some("auth-int"));

    // Sem qop: Digest legado (RFC 2069).
    assert_eq!(select_qop(""), None);
}

#[test]
fn test_parse_input_proxy_channels() {
    use onliview::camera::isapi::parse_input_proxy_channels;

    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <InputProxyChannelList version="2.0">
      <InputProxyChannel>
        <id>1</id>
        <name>ENTRADA PRINCIPAL</name>
        <sourceInputPortDescriptor>
          <proxyProtocol>HIKVISION</proxyProtocol>
          <ipAddress>10.10.1.10</ipAddress>
          <managePortNo>8000</managePortNo>
        </sourceInputPortDescriptor>
      </InputProxyChannel>
      <InputProxyChannel>
        <id>2</id>
        <name>GARAGEM</name>
        <sourceInputPortDescriptor>
          <ipAddress>10.10.1.11</ipAddress>
        </sourceInputPortDescriptor>
      </InputProxyChannel>
      <InputProxyChannel>
        <id>3</id>
        <name>CANAL ANALOGICO</name>
        <sourceInputPortDescriptor>
          <ipAddress></ipAddress>
        </sourceInputPortDescriptor>
      </InputProxyChannel>
      <InputProxyChannel>
        <name>SEM ID - DEVE SER DESCARTADO</name>
      </InputProxyChannel>
    </InputProxyChannelList>"#;

    let ch = parse_input_proxy_channels(xml);
    assert_eq!(ch.len(), 3, "canal sem id legivel deve ser descartado");

    assert_eq!(ch[0].id, 1);
    assert_eq!(ch[0].name, "ENTRADA PRINCIPAL");
    assert_eq!(ch[0].ip_address, Some("10.10.1.10".to_string()));

    assert_eq!(ch[1].id, 2);
    assert_eq!(ch[1].ip_address, Some("10.10.1.11".to_string()));

    // IP vazio => canal analogico/local, nao "cadastro faltando".
    assert_eq!(ch[2].id, 3);
    assert_eq!(ch[2].ip_address, None);
}

#[test]
fn test_parse_channel_status_unknown_is_not_offline() {
    use onliview::camera::isapi::parse_channel_status;

    let xml = r#"<InputProxyChannelStatusList>
      <InputProxyChannelStatus><id>1</id><online>true</online></InputProxyChannelStatus>
      <InputProxyChannelStatus><id>2</id><online>false</online></InputProxyChannelStatus>
      <InputProxyChannelStatus><id>3</id></InputProxyChannelStatus>
    </InputProxyChannelStatusList>"#;

    let st = parse_channel_status(xml);
    assert_eq!(st.get(&1), Some(&Some(true)));
    assert_eq!(st.get(&2), Some(&Some(false)));

    // Firmware que nao informa <online>: desconhecido, jamais "offline" —
    // afirmar offline sem base viraria alarme falso na tela do tecnico.
    assert_eq!(st.get(&3), Some(&None));
}

#[test]
fn test_parse_search_result_skips_malformed_and_detects_more() {
    use onliview::camera::isapi::parse_search_result;

    let xml = r#"<CMSearchResult>
      <searchID>abc</searchID>
      <responseStatus>true</responseStatus>
      <responseStatusStrg>MORE</responseStatusStrg>
      <numOfMatches>3</numOfMatches>
      <matchList>
        <searchMatchItem>
          <trackID>101</trackID>
          <timeSpan><startTime>2026-08-27T10:00:00Z</startTime><endTime>2026-08-27T10:30:00Z</endTime></timeSpan>
        </searchMatchItem>
        <searchMatchItem>
          <trackID>101</trackID>
          <timeSpan><startTime>2026-08-27T11:00:00Z</startTime></timeSpan>
        </searchMatchItem>
        <searchMatchItem>
          <trackID>101</trackID>
          <timeSpan><startTime>2026-08-27T12:00:00Z</startTime><endTime>2026-08-27T12:45:00Z</endTime></timeSpan>
        </searchMatchItem>
      </matchList>
    </CMSearchResult>"#;

    let page = parse_search_result(xml);
    assert_eq!(page.segments.len(), 2, "item sem endTime deve ser pulado, nao abortar a pagina");
    assert_eq!(page.segments[0].start, "2026-08-27T10:00:00Z");
    assert_eq!(page.segments[1].end, "2026-08-27T12:45:00Z");
    assert!(page.has_more, "responseStatusStrg=MORE indica pagina truncada");
}

#[test]
fn test_parse_search_result_no_matches() {
    use onliview::camera::isapi::parse_search_result;

    // Camera que nao gravou nada no periodo: resultado vazio e valido,
    // nao um erro — e distingue "nao gravou" de "nao consegui perguntar".
    let xml = r#"<CMSearchResult>
      <responseStatusStrg>NO MATCHES</responseStatusStrg>
      <numOfMatches>0</numOfMatches>
      <matchList></matchList>
    </CMSearchResult>"#;

    let page = parse_search_result(xml);
    assert!(page.segments.is_empty());
    assert!(!page.has_more);

    // Resposta completamente ilegivel nao pode panicar.
    let garbage = parse_search_result("<<< nao e xml >>>");
    assert!(garbage.segments.is_empty());
}

#[test]
fn test_coverage_ratio_merges_overlaps_and_clips_to_period() {
    use onliview::camera::isapi::RecordingSegment;
    use onliview::camera::recording::{coverage_ratio, parse_isapi_time};

    let start = parse_isapi_time("2026-08-27T00:00:00Z").unwrap();
    let end = parse_isapi_time("2026-08-28T00:00:00Z").unwrap(); // 24h

    let seg = |a: &str, b: &str| RecordingSegment { start: a.to_string(), end: b.to_string() };

    // Sem gravacao alguma.
    assert_eq!(coverage_ratio(&[], start, end), 0.0);

    // 6h de 24h = 25%.
    let quarter = vec![seg("2026-08-27T00:00:00Z", "2026-08-27T06:00:00Z")];
    assert!((coverage_ratio(&quarter, start, end) - 0.25).abs() < 0.001);

    // Segmentos sobrepostos nao podem somar duas vezes: 00-06 e 03-09 sao 9h, nao 12h.
    let overlapping = vec![
        seg("2026-08-27T00:00:00Z", "2026-08-27T06:00:00Z"),
        seg("2026-08-27T03:00:00Z", "2026-08-27T09:00:00Z"),
    ];
    let ratio = coverage_ratio(&overlapping, start, end);
    assert!((ratio - 0.375).abs() < 0.001, "esperado 9h/24h=0.375, obtido {}", ratio);

    // Segmento que extrapola o periodo e recortado, nunca passa de 100%.
    let overflowing = vec![seg("2026-08-26T00:00:00Z", "2026-08-29T00:00:00Z")];
    assert!((coverage_ratio(&overflowing, start, end) - 1.0).abs() < 0.001);

    // Segmento inteiramente fora do periodo nao conta.
    let outside = vec![seg("2026-08-20T00:00:00Z", "2026-08-21T00:00:00Z")];
    assert_eq!(coverage_ratio(&outside, start, end), 0.0);

    // Horario ilegivel e ignorado sem panicar.
    let broken = vec![seg("nao-e-data", "tambem-nao")];
    assert_eq!(coverage_ratio(&broken, start, end), 0.0);
}

#[test]
fn test_parse_isapi_time_accepts_firmware_variants() {
    use onliview::camera::recording::parse_isapi_time;

    assert!(parse_isapi_time("2026-08-27T10:00:00Z").is_some());
    assert!(parse_isapi_time("2026-08-27T10:00:00-03:00").is_some());
    // Firmwares que omitem o fuso: tratado como UTC em vez de descartado.
    assert!(parse_isapi_time("2026-08-27T10:00:00").is_some());
    assert!(parse_isapi_time("").is_none());
    assert!(parse_isapi_time("qualquer coisa").is_none());

    // Mesmo instante expresso com e sem offset deve bater.
    let utc = parse_isapi_time("2026-08-27T13:00:00Z").unwrap();
    let off = parse_isapi_time("2026-08-27T10:00:00-03:00").unwrap();
    assert_eq!(utc, off);
}

#[test]
fn test_match_camera_by_ip_and_track_id() {
    use onliview::camera::recording::{match_camera_by_ip, track_id_for_channel};

    // Convencao Hikvision de track do fluxo principal.
    assert_eq!(track_id_for_channel(1), 101);
    assert_eq!(track_id_for_channel(2), 201);
    assert_eq!(track_id_for_channel(16), 1601);

    let db_path = "test_match_ip.db";
    let _ = std::fs::remove_file(db_path);
    let db = Database::new(db_path).expect("Failed to init DB");
    let created = db.create_cameras_batch(BatchCreateCamerasInput {
        devices: vec![
            BatchDeviceItem {
                name: "Portaria".to_string(),
                host: "10.10.1.10".to_string(),
                rtsp_port: 554,
                http_port: Some(80),
                custom_rtsp_url: None,
                device_name: None,
                osd: None,
                device_type: None,
            },
        ],
        username: "admin".to_string(),
        password: Some("x".to_string()),
        stream_profile: "main".to_string(),
    }).expect("batch failed");

    assert!(match_camera_by_ip("10.10.1.10", &created).is_some());
    assert!(match_camera_by_ip("10.10.1.99", &created).is_none());

    // Espacos em volta do valor vindo do XML nao quebram o casamento.
    assert!(match_camera_by_ip("  10.10.1.10  ", &created).is_some());

    // Zeros a esquerda NAO casam, e isso e proposital: "010" e ambiguo entre
    // decimal e octal, e o parser de Ipv4Addr do Rust o rejeita em vez de
    // adivinhar. Preferimos reportar "nao cadastrada" a casar a camera errada.
    assert!(match_camera_by_ip("010.010.001.010", &created).is_none());

    // Canal analogico (sem IP) nunca casa.
    assert!(match_camera_by_ip("", &created).is_none());
    assert!(match_camera_by_ip("   ", &created).is_none());

    let _ = std::fs::remove_file(db_path);
}
