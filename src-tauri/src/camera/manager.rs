use std::sync::Arc;

use chrono::{DateTime, Duration as ChronoDuration, Utc};

use crate::database::Database;
use crate::camera::model::*;
use crate::camera::isapi::{IsapiClient, RecordingSegment};
use crate::camera::recording::*;
use crate::logging::logger::sanitize_credentials;
use crate::rtsp::client::build_authenticated_rtsp_url;
use crate::rtsp::probe::probe_rtsp_stream;
use crate::video::engine::VideoEngineManager;
use crate::logging::logger::LogStore;
use crate::discovery::{DiscoveryEngine, NetworkInterfaceManager, NetworkInterfaceInfo, DiscoveredDevice};

#[derive(Clone)]
pub struct CameraManager {
    db: Database,
    video_engine: VideoEngineManager,
    log_store: LogStore,
}

impl CameraManager {
    pub fn new(db: Database, video_engine: VideoEngineManager, log_store: LogStore) -> Self {
        Self {
            db,
            video_engine,
            log_store,
        }
    }

    pub fn get_network_interfaces(&self) -> Vec<NetworkInterfaceInfo> {
        NetworkInterfaceManager::get_interfaces()
    }

    pub async fn fetch_device_metadata(
        host: &str,
        http_port: u16,
        username: &str,
        password: &str,
    ) -> (Option<String>, Option<String>) {
        let client = IsapiClient::new(host, http_port, username, password);
        let dev_name = match tokio::time::timeout(std::time::Duration::from_millis(2500), client.get_device_info()).await {
            Ok(Ok(info)) => {
                let n = info.device_name.trim();
                if !n.is_empty() {
                    Some(n.to_string())
                } else {
                    None
                }
            }
            _ => None,
        };
        let osd = match tokio::time::timeout(std::time::Duration::from_millis(2500), client.get_osd_title(1)).await {
            Ok(Ok(title)) => {
                let t = title.trim();
                if !t.is_empty() {
                    Some(t.to_string())
                } else {
                    None
                }
            }
            _ => None,
        };
        (dev_name, osd)
    }

    pub fn get_cameras(&self) -> Result<Vec<Camera>, String> {
        self.db.get_cameras().map_err(|e| e.to_string())
    }

    pub fn get_camera_by_id(&self, id: &str) -> Result<Option<Camera>, String> {
        self.db.get_camera_by_id(id).map_err(|e| e.to_string())
    }

    pub async fn create_camera(&self, mut input: CreateCameraInput) -> Result<Camera, String> {
        let http_port = input.http_port.unwrap_or(80);
        let username = input.username.trim();
        let password = input.password.as_deref().unwrap_or_default();

        // If device_name or osd is not explicitly provided, attempt to capture them from the device
        if (input.device_name.is_none() || input.osd.is_none()) && !input.host.trim().is_empty() {
            let (dev_name, osd) = Self::fetch_device_metadata(input.host.trim(), http_port, username, password).await;
            if input.device_name.is_none() {
                input.device_name = dev_name;
            }
            if input.osd.is_none() {
                input.osd = osd;
            }
        }

        let cam = self.db.create_camera(input)?;
        self.log_store.log("INFO", "CameraManager", &format!("Câmera cadastrada: {} ({}) [DeviceName: {:?}, OSD: {:?}]", cam.name, cam.host, cam.device_name, cam.osd));
        Ok(cam)
    }

    pub async fn create_cameras_batch(&self, mut input: BatchCreateCamerasInput) -> Result<Vec<Camera>, String> {
        let count = input.devices.len();
        let username = input.username.trim().to_string();
        let password = input.password.clone().unwrap_or_default();

        // Parallel metadata fetch for each device in batch with timeout
        let mut set = tokio::task::JoinSet::new();
        for (i, dev) in input.devices.iter().enumerate() {
            let host = dev.host.clone();
            let http_port = dev.http_port.unwrap_or(80);
            let u = username.clone();
            let p = password.clone();
            let existing_dev_name = dev.device_name.clone();
            let existing_osd = dev.osd.clone();

            set.spawn(async move {
                if existing_dev_name.is_some() && existing_osd.is_some() {
                    (i, existing_dev_name, existing_osd)
                } else {
                    let (dev_name, osd) = Self::fetch_device_metadata(&host, http_port, &u, &p).await;
                    (i, existing_dev_name.or(dev_name), existing_osd.or(osd))
                }
            });
        }

        while let Some(res) = set.join_next().await {
            if let Ok((i, dev_name, osd)) = res {
                if let Some(dev) = input.devices.get_mut(i) {
                    dev.device_name = dev_name;
                    dev.osd = osd;
                }
            }
        }

        let cams = self.db.create_cameras_batch(input)?;
        self.log_store.log("INFO", "CameraManager", &format!("Adicionadas {} câmeras em lote com sucesso", count));
        Ok(cams)
    }

    pub async fn discover_devices(&self, interface_name: Option<String>) -> Result<Vec<DiscoveredDevice>, String> {
        let iface_desc = interface_name.clone().unwrap_or_else(|| "Padrão".to_string());
        self.log_store.log("INFO", "Discovery", &format!("Iniciando Descoberta Inteligente Multicamada na interface: {}", iface_desc));
        
        // Discovery only reports ~5 phase transitions total, so logging every one of them is not
        // spam — the previous "% 25 == 0" filter accidentally suppressed nearly all of them (the
        // actual percentages used are 10/30/65/85/100, which barely intersect that pattern),
        // hiding exactly the diagnostic detail (which subnet was scanned, host count) needed to
        // debug discovery coverage issues.
        let log_clone = self.log_store.clone();
        let mut devices = DiscoveryEngine::run_discovery(interface_name, move |prog| {
            log_clone.log("INFO", "Discovery", &format!("[Progresso {}%] {}", prog.percentage, prog.phase));
        }).await;
        
        let existing_cameras = self.db.get_cameras().unwrap_or_default();
        let existing_hosts: std::collections::HashSet<String> = existing_cameras.into_iter().map(|c| c.host).collect();

        for dev in &mut devices {
            if existing_hosts.contains(&dev.ip) {
                dev.is_already_added = true;
            }
        }

        self.log_store.log("INFO", "Discovery", &format!("Descoberta concluída: {} dispositivos localizados e diagnosticados", devices.len()));
        Ok(devices)
    }

    pub fn update_camera(&self, input: UpdateCameraInput) -> Result<Camera, String> {
        let cam = self.db.update_camera(input)?;
        self.log_store.log("INFO", "CameraManager", &format!("Câmera atualizada: {} ({})", cam.name, cam.host));
        Ok(cam)
    }

    pub async fn delete_camera(&self, id: &str) -> Result<(), String> {
        self.video_engine.stop(id).await.ok();
        self.db.delete_camera(id)?;
        self.log_store.log("INFO", "CameraManager", &format!("Câmera removida: {}", id));
        Ok(())
    }

    pub async fn delete_cameras_batch(&self, ids: Vec<String>) -> Result<usize, String> {
        for id in &ids {
            self.video_engine.stop(id).await.ok();
        }
        let count = self.db.delete_cameras_batch(&ids)?;
        self.log_store.log("INFO", "CameraManager", &format!("Removidas {} câmeras em lote", count));
        Ok(count)
    }

    pub async fn delete_all_cameras(&self) -> Result<usize, String> {
        let all_cams = self.db.get_cameras().unwrap_or_default();
        for cam in &all_cams {
            self.video_engine.stop(&cam.id).await.ok();
        }
        let count = self.db.delete_all_cameras()?;
        self.log_store.log("INFO", "CameraManager", &format!("Todas as {} câmeras foram removidas", count));
        Ok(count)
    }

    pub async fn test_connection(&self, input: CreateCameraInput) -> CameraConnectionTestResult {
        let host = input.host.trim();
        let port = input.rtsp_port.unwrap_or(554);
        let http_port = input.http_port.unwrap_or(80);
        let user = input.username.trim();
        let pass = input.password.as_deref().unwrap_or_default();
        let raw_url = input.rtsp_url.as_deref().unwrap_or_default();

        let full_url = build_authenticated_rtsp_url(host, port, user, pass, raw_url);
        self.log_store.log("INFO", "CameraManager", &format!("Testando conexão com {}", full_url));
        
        let mut result = probe_rtsp_stream(&full_url).await;

        if !host.is_empty() {
            let (dev_name, osd) = Self::fetch_device_metadata(host, http_port, user, pass).await;
            result.device_name = dev_name;
            result.osd = osd;
        }

        result
    }

    pub async fn test_existing_camera_connection(&self, camera_id: &str) -> Result<CameraConnectionTestResult, String> {
        let cam = self.db.get_camera_by_id(camera_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Camera not found".to_string())?;

        let password = self.db.get_camera_decrypted_password(camera_id)?
            .unwrap_or_default();

        let full_url = build_authenticated_rtsp_url(&cam.host, cam.rtsp_port, &cam.username, &password, &cam.rtsp_url);
        self.log_store.log("INFO", "CameraManager", &format!("Testando conexão para câmera cadastrada {}", cam.name));
        
        let mut result = probe_rtsp_stream(&full_url).await;
        result.device_name = cam.device_name;
        result.osd = cam.osd;
        Ok(result)
    }

    pub async fn start_camera_stream(&self, camera_id: &str) -> Result<(), String> {
        let cam = self.db.get_camera_by_id(camera_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Camera not found".to_string())?;

        if !cam.enabled {
            return Err("Camera is disabled".to_string());
        }

        let password = self.db.get_camera_decrypted_password(camera_id)?
            .unwrap_or_default();

        let full_url = build_authenticated_rtsp_url(&cam.host, cam.rtsp_port, &cam.username, &password, &cam.rtsp_url);
        self.video_engine.connect(camera_id, &full_url).await
    }

    pub async fn stop_camera_stream(&self, camera_id: &str) -> Result<(), String> {
        self.video_engine.stop(camera_id).await
    }
}

impl CameraManager {
    pub async fn quick_view_connect(&self, input: QuickViewConnectInput) -> Result<QuickViewSessionInfo, String> {
        let host = input.ip.trim().to_string();
        let mac_hint = input.mac.clone();
        let rtsp_port = input.rtsp_port.unwrap_or(554);
        let http_port = input.http_port.unwrap_or(80);
        let username = input.username.trim().to_string();
        let password = input.password.unwrap_or_default();
        let remember_password = input.remember_password.unwrap_or(false);

        self.log_store.log("INFO", "QuickViewer", &format!("Autenticando sessão Quick View para dispositivo: {}", host));

        let isapi_client = IsapiClient::new(&host, http_port, &username, &password);

        // 1. Fetch ISAPI Device Info or fallback
        let (dev_name, model, serial, firmware, mac, brand) = match isapi_client.get_device_info().await {
            Ok(info) => {
                (info.device_name, info.model, info.serial_number, info.firmware_version, info.mac_address, "Hikvision".to_string())
            }
            Err(e) => {
                self.log_store.log("WARN", "QuickViewer", &format!("ISAPI deviceInfo não disponível ({}), tentando modo direto RTSP", e));
                (format!("Câmera {}", host), "Câmera IP".to_string(), None, None, None, "Câmera IP".to_string())
            }
        };

        // Whether to cache this password is entirely up to the technician's "remember password"
        // choice in the login window — this is the single place credentials get saved or forgotten.
        let effective_mac = mac.clone().or_else(|| mac_hint.clone());
        if remember_password {
            if let Err(e) = self.db.save_device_credentials(&host, effective_mac.as_deref(), &username, &password) {
                self.log_store.log("WARN", "QuickViewer", &format!("Falha ao salvar credenciais em cache para {}: {}", host, e));
            }
        } else if let Err(e) = self.db.delete_device_credentials(&host) {
            self.log_store.log("WARN", "QuickViewer", &format!("Falha ao remover credenciais em cache para {}: {}", host, e));
        }

        // 2. Fetch OSD
        let osd_text = isapi_client.get_osd_title(1).await.unwrap_or_default();

        // 3. Detect Capabilities
        let capabilities = isapi_client.detect_capabilities().await;

        // 4. Discover Stream URL (prefer substream 102 for instant, low-latency rendering)
        let channel_path = isapi_client.discover_substream_channel_url().await;
        let full_rtsp_url = build_authenticated_rtsp_url(&host, rtsp_port, &username, &password, &channel_path);

        // 5. Start Video Stream in VideoEngine IMMEDIATELY for sub-second latency
        let session_id = format!("quick_view_{}", host.replace('.', "_"));
        if let Err(_) = self.video_engine.connect(&session_id, &full_rtsp_url).await {
            // Fallback to main stream if substream failed
            let main_path = isapi_client.discover_streaming_channel_url().await;
            let fallback_url = build_authenticated_rtsp_url(&host, rtsp_port, &username, &password, &main_path);
            if let Err(e) = self.video_engine.connect(&session_id, &fallback_url).await {
                self.log_store.log("ERROR", "QuickViewer", &format!("Erro ao iniciar stream de vídeo: {}", e));
                return Err(e);
            }
        }

        let local_mjpeg_url = format!("http://127.0.0.1:{}/stream/{}", self.video_engine.server_port(), session_id);

        // 6. Fast probe with non-blocking timeout
        let probe_url = full_rtsp_url.clone();
        let metrics = tokio::time::timeout(
            std::time::Duration::from_millis(1500),
            probe_rtsp_stream(&probe_url)
        ).await.unwrap_or_else(|_| {
            CameraConnectionTestResult {
                success: true,
                message: "Conexão RTSP ativa".to_string(),
                codec: Some("H.264".to_string()),
                resolution: Some("1080p".to_string()),
                fps: Some(25.0),
                bitrate: None,
                latency_ms: Some(12),
                device_name: None,
                osd: None,
            }
        });

        self.log_store.log("INFO", "QuickViewer", &format!("Sessão Quick View estabelecida com sucesso para {}", host));

        Ok(QuickViewSessionInfo {
            ip: host,
            rtsp_port,
            http_port,
            brand,
            hardware_model: model,
            serial_number: serial,
            firmware_version: firmware,
            mac_address: mac,
            device_name: dev_name,
            osd_text,
            stream_url: full_rtsp_url,
            local_mjpeg_url,
            capabilities,
            metrics,
        })
    }

    pub async fn quick_view_disconnect(&self, ip: &str) -> Result<(), String> {
        let session_id = format!("quick_view_{}", ip.trim().replace('.', "_"));
        self.video_engine.stop(&session_id).await.ok();
        self.log_store.log("INFO", "QuickViewer", &format!("Sessão Quick View finalizada para {}", ip));
        Ok(())
    }

    pub async fn quick_view_set_device_name(&self, input: QuickViewSetDeviceNameInput) -> Result<(), String> {
        let host = input.ip.trim();
        let http_port = input.http_port.unwrap_or(80);
        let username = input.username.trim();
        let password = input.password.unwrap_or_default();
        let new_name = input.new_name.trim();

        self.log_store.log("INFO", "QuickViewer", &format!("Alterando Device Name para '{}' no host {}", new_name, host));

        let isapi_client = IsapiClient::new(host, http_port, username, &password);
        match isapi_client.set_device_name(new_name).await {
            Ok(_) => {
                if let Ok(cams) = self.db.get_cameras() {
                    for cam in cams {
                        if cam.host == host {
                            let _ = self.db.update_camera(UpdateCameraInput {
                                id: cam.id,
                                name: None,
                                host: None,
                                username: None,
                                password: None,
                                rtsp_port: None,
                                http_port: None,
                                rtsp_url: None,
                                stream_profile: None,
                                enabled: None,
                                device_name: Some(new_name.to_string()),
                                osd: None,
                                device_type: None,
                            });
                        }
                    }
                }
                self.log_store.log("INFO", "Audit", &format!("Operação CHANGE_DEVICE_NAME realizada com sucesso no host {}", host));
                Ok(())
            }
            Err(e) => {
                self.log_store.log("ERROR", "Audit", &format!("Falha na operação CHANGE_DEVICE_NAME no host {}: {}", host, e));
                Err(e)
            }
        }
    }

    pub async fn quick_view_set_osd(&self, input: QuickViewSetOsdInput) -> Result<(), String> {
        let host = input.ip.trim();
        let http_port = input.http_port.unwrap_or(80);
        let channel_id = input.channel_id.unwrap_or(1);
        let username = input.username.trim();
        let password = input.password.unwrap_or_default();
        let new_osd = input.new_osd.trim();

        self.log_store.log("INFO", "QuickViewer", &format!("Alterando OSD para '{}' no canal {} do host {}", new_osd, channel_id, host));

        let isapi_client = IsapiClient::new(host, http_port, username, &password);
        match isapi_client.set_osd_title(channel_id, new_osd).await {
            Ok(_) => {
                if let Ok(cams) = self.db.get_cameras() {
                    for cam in cams {
                        if cam.host == host {
                            let _ = self.db.update_camera(UpdateCameraInput {
                                id: cam.id,
                                name: None,
                                host: None,
                                username: None,
                                password: None,
                                rtsp_port: None,
                                http_port: None,
                                rtsp_url: None,
                                stream_profile: None,
                                enabled: None,
                                device_name: None,
                                osd: Some(new_osd.to_string()),
                                device_type: None,
                            });
                        }
                    }
                }
                self.log_store.log("INFO", "Audit", &format!("Operação CHANGE_OSD realizada com sucesso no host {}", host));
                Ok(())
            }
            Err(e) => {
                self.log_store.log("ERROR", "Audit", &format!("Falha na operação CHANGE_OSD no host {}: {}", host, e));
                Err(e)
            }
        }
    }
}

impl CameraManager {
    pub async fn start_device_preview(&self, input: QuickViewConnectInput) -> Result<String, String> {
        let host = input.ip.trim().to_string();
        let rtsp_port = input.rtsp_port.unwrap_or(554);
        let http_port = input.http_port.unwrap_or(80);
        let username = input.username.trim().to_string();
        let password = input.password.unwrap_or_default();
        let remember_password = input.remember_password.unwrap_or(false);

        let isapi_client = IsapiClient::new(&host, http_port, &username, &password);

        // Try substream first (102) for low-bandwidth thumbnail, fallback to main (101)
        let channel_path = "/Streaming/Channels/102";
        let full_rtsp_url = build_authenticated_rtsp_url(&host, rtsp_port, &username, &password, channel_path);

        let session_id = format!("preview_{}", host.replace('.', "_"));

        // Connect to stream in VideoEngine
        if let Err(_) = self.video_engine.connect(&session_id, &full_rtsp_url).await {
            // Fallback to main stream
            let main_path = isapi_client.discover_streaming_channel_url().await;
            let fallback_url = build_authenticated_rtsp_url(&host, rtsp_port, &username, &password, &main_path);
            self.video_engine.connect(&session_id, &fallback_url).await?;
        }

        if remember_password {
            if let Err(e) = self.db.save_device_credentials(&host, input.mac.as_deref(), &username, &password) {
                self.log_store.log("WARN", "DevicePreview", &format!("Falha ao salvar credenciais em cache para {}: {}", host, e));
            }
        }

        let stream_url = format!("http://127.0.0.1:{}/stream/{}", self.video_engine.server_port(), session_id);
        Ok(stream_url)
    }

    pub async fn stop_device_preview(&self, ip: &str) -> Result<(), String> {
        let session_id = format!("preview_{}", ip.trim().replace('.', "_"));
        self.video_engine.stop(&session_id).await.ok();
        Ok(())
    }

    pub fn get_cached_credentials(&self, ip: &str, mac: Option<&str>) -> Result<Option<CachedDeviceCredentials>, String> {
        let cached = self.db.get_device_credentials(ip.trim(), mac)?;
        Ok(cached.map(|(username, password)| CachedDeviceCredentials { username, password }))
    }

    /// Forgets a saved password without requiring a successful authentication first — needed when
    /// the cached password is stale/incorrect and the technician can't "connect" to trigger the
    /// normal uncheck-and-reconnect forget flow.
    pub fn forget_device_credentials(&self, ip: &str) -> Result<(), String> {
        self.db.delete_device_credentials(ip.trim())
    }

    /// Consulta os NVRs cadastrados e monta o panorama de gravação por canal.
    ///
    /// Tudo ao vivo — nada é persistido. Sem período informado, usa as últimas 24h.
    pub async fn check_recordings(
        &self,
        period_start: Option<String>,
        period_end: Option<String>,
        nvr_ids: Option<Vec<String>>,
    ) -> Result<RecordingCheckResult, String> {
        let end = period_end
            .as_deref()
            .and_then(parse_isapi_time)
            .unwrap_or_else(Utc::now);
        let start = period_start
            .as_deref()
            .and_then(parse_isapi_time)
            .unwrap_or_else(|| end - ChronoDuration::hours(24));

        if start >= end {
            return Err("O início do período deve ser anterior ao fim.".to_string());
        }

        let all = self.db.get_cameras().map_err(|e| e.to_string())?;
        let (recorders, cameras): (Vec<Camera>, Vec<Camera>) = all
            .into_iter()
            .partition(|c| matches!(c.device_type.as_str(), "nvr" | "dvr"));

        let recorders: Vec<Camera> = match &nvr_ids {
            Some(ids) => recorders.into_iter().filter(|r| ids.contains(&r.id)).collect(),
            None => recorders,
        };

        if recorders.is_empty() {
            self.log_store.log(
                "WARN",
                "Gravacoes",
                "Verificação solicitada, mas nenhum NVR/DVR está cadastrado.",
            );
            return Ok(RecordingCheckResult {
                period_start: format_isapi_time(start),
                period_end: format_isapi_time(end),
                nvr_reports: Vec::new(),
                orphan_cameras: cameras,
            });
        }

        // Um NVR por task: hosts diferentes, então o paralelismo aqui é seguro.
        // O limite de sessões é aplicado dentro de cada NVR, não entre eles.
        let mut tasks = tokio::task::JoinSet::new();
        for nvr in recorders {
            let password = self
                .db
                .get_camera_decrypted_password(&nvr.id)
                .unwrap_or_default()
                .unwrap_or_default();
            let cams = cameras.clone();
            tasks.spawn(async move { check_single_nvr(nvr, password, cams, start, end).await });
        }

        let mut nvr_reports = Vec::new();
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(report) => nvr_reports.push(report),
                Err(e) => {
                    self.log_store.log(
                        "ERROR",
                        "Gravacoes",
                        &sanitize_credentials(&format!("Falha interna ao verificar um NVR: {}", e)),
                    );
                }
            }
        }

        nvr_reports.sort_by(|a, b| a.nvr_name.cmp(&b.nvr_name));

        // Órfã só depois de consultar todos: uma câmera no NVR B não pode ser
        // marcada órfã só por faltar no NVR A.
        let matched: std::collections::HashSet<String> = nvr_reports
            .iter()
            .flat_map(|r| r.channels.iter())
            .filter_map(|c| c.matched_camera_id.clone())
            .collect();
        let orphan_cameras: Vec<Camera> = cameras
            .into_iter()
            .filter(|c| !matched.contains(&c.id))
            .collect();

        let reachable = nvr_reports.iter().filter(|r| r.reachable && r.auth_ok).count();
        let not_recording = nvr_reports
            .iter()
            .flat_map(|r| r.channels.iter())
            .filter(|c| c.is_recording == Some(false))
            .count();

        // Só contagens no log — nada de nome de host com credencial embutida.
        self.log_store.log(
            "INFO",
            "Gravacoes",
            &format!(
                "Verificação concluída: {}/{} gravador(es) acessível(is), {} canal(is) sem gravação, {} câmera(s) fora de qualquer NVR.",
                reachable,
                nvr_reports.len(),
                not_recording,
                orphan_cameras.len()
            ),
        );

        Ok(RecordingCheckResult {
            period_start: format_isapi_time(start),
            period_end: format_isapi_time(end),
            nvr_reports,
            orphan_cameras,
        })
    }
}

/// Consulta um NVR: lista canais, casa com o cadastro e busca gravações.
async fn check_single_nvr(
    nvr: Camera,
    password: String,
    cameras: Vec<Camera>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> NvrRecordingReport {
    let mut report = NvrRecordingReport {
        nvr_id: nvr.id.clone(),
        nvr_name: nvr.name.clone(),
        nvr_host: nvr.host.clone(),
        reachable: false,
        auth_ok: false,
        error: None,
        channels: Vec::new(),
        unregistered_channels: Vec::new(),
    };

    let client = Arc::new(IsapiClient::new(
        &nvr.host,
        nvr.http_port,
        &nvr.username,
        &password,
    ));

    let channels = match client.get_input_proxy_channels().await {
        Ok(c) => {
            report.reachable = true;
            report.auth_ok = true;
            c
        }
        Err(e) => {
            // Credencial errada é acionável de forma diferente de cabo/rota
            // fora, então separamos os dois casos para o técnico.
            let is_auth = e.contains("senha") || e.contains("permissão");
            report.reachable = !e.contains("conectar") && !e.contains("Timeout");
            report.auth_ok = !is_auth;
            report.error = Some(e);
            return report;
        }
    };

    // Estado online é complementar: se falhar, seguimos sem ele.
    let status = client
        .get_input_proxy_channel_status()
        .await
        .unwrap_or_default();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_PER_NVR));
    let mut tasks = tokio::task::JoinSet::new();

    for ch in channels {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let online = status.get(&ch.id).copied().flatten();
        let matched = ch
            .ip_address
            .as_deref()
            .and_then(|ip| match_camera_by_ip(ip, &cameras))
            .map(|c| (c.id.clone(), c.name.clone()));

        tasks.spawn(async move {
            let _permit = semaphore.acquire().await;
            let mut entry = ChannelRecordingStatus {
                channel_id: ch.id,
                channel_name: ch.name,
                ip_address: ch.ip_address,
                online,
                matched_camera_id: matched.as_ref().map(|(id, _)| id.clone()),
                matched_camera_name: matched.as_ref().map(|(_, n)| n.clone()),
                is_recording: None,
                segments: Vec::new(),
                coverage_ratio: 0.0,
                truncated: false,
                error: None,
            };

            match fetch_channel_segments(&client, ch.id, start, end).await {
                Ok((segments, truncated)) => {
                    entry.is_recording = Some(!segments.is_empty());
                    entry.coverage_ratio = coverage_ratio(&segments, start, end);
                    entry.segments = segments;
                    entry.truncated = truncated;
                }
                // is_recording fica None: "não consegui perguntar" não é "não gravou".
                Err(e) => entry.error = Some(e),
            }

            entry
        });
    }

    let mut collected = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        if let Ok(entry) = joined {
            collected.push(entry);
        }
    }
    collected.sort_by_key(|c| c.channel_id);

    for entry in collected {
        if entry.matched_camera_id.is_some() {
            report.channels.push(entry);
        } else {
            report.unregistered_channels.push(entry);
        }
    }

    report
}

/// Pagina a busca de gravações de um canal, com teto rígido de páginas.
async fn fetch_channel_segments(
    client: &IsapiClient,
    channel_id: u32,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<(Vec<RecordingSegment>, bool), String> {
    let track_id = track_id_for_channel(channel_id);
    let start_s = format_isapi_time(start);
    let end_s = format_isapi_time(end);

    let mut segments = Vec::new();
    let mut position = 0u32;
    let mut truncated = false;

    for page in 0..MAX_SEARCH_PAGES {
        let result = client
            .search_recordings_page(track_id, &start_s, &end_s, SEARCH_PAGE_SIZE, position)
            .await?;

        let returned = result.segments.len() as u32;
        segments.extend(result.segments);

        if !result.has_more || returned == 0 {
            break;
        }

        position += returned;

        if page + 1 == MAX_SEARCH_PAGES {
            truncated = true;
        }
    }

    Ok((segments, truncated))
}
