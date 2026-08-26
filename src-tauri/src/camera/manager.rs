use std::time::Duration;
use crate::database::Database;
use crate::camera::model::*;
use crate::rtsp::client::build_authenticated_rtsp_url;
use crate::rtsp::probe::probe_rtsp_stream;
use crate::video::engine::VideoEngineManager;
use crate::logging::logger::LogStore;
use crate::onvif::discovery::OnvifDiscovery;

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

    pub fn get_cameras(&self) -> Result<Vec<Camera>, String> {
        self.db.get_cameras().map_err(|e| e.to_string())
    }

    pub fn get_camera_by_id(&self, id: &str) -> Result<Option<Camera>, String> {
        self.db.get_camera_by_id(id).map_err(|e| e.to_string())
    }

    pub fn create_camera(&self, input: CreateCameraInput) -> Result<Camera, String> {
        let cam = self.db.create_camera(input)?;
        self.log_store.log("INFO", "CameraManager", &format!("Câmera cadastrada: {} ({})", cam.name, cam.host));
        Ok(cam)
    }

    pub fn create_cameras_batch(&self, input: BatchCreateCamerasInput) -> Result<Vec<Camera>, String> {
        let count = input.devices.len();
        let cams = self.db.create_cameras_batch(input)?;
        self.log_store.log("INFO", "CameraManager", &format!("Adicionadas {} câmeras em lote com sucesso", count));
        Ok(cams)
    }

    pub async fn discover_devices(&self) -> Result<Vec<DiscoveredDevice>, String> {
        self.log_store.log("INFO", "Discovery", "Iniciando varredura de rede local por dispositivos ONVIF/CFTV");
        
        let mut devices = OnvifDiscovery::discover_devices(Duration::from_millis(2500)).await?;
        
        let existing_cameras = self.db.get_cameras().unwrap_or_default();
        let existing_hosts: std::collections::HashSet<String> = existing_cameras.into_iter().map(|c| c.host).collect();

        for dev in &mut devices {
            if existing_hosts.contains(&dev.ip) {
                dev.is_already_added = true;
            }
        }

        self.log_store.log("INFO", "Discovery", &format!("Varredura concluída: {} dispositivos localizados", devices.len()));
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

    pub async fn test_connection(&self, input: CreateCameraInput) -> CameraConnectionTestResult {
        let host = input.host.trim();
        let port = input.rtsp_port.unwrap_or(554);
        let user = input.username.trim();
        let pass = input.password.unwrap_or_default();
        let raw_url = input.rtsp_url.unwrap_or_default();

        let full_url = build_authenticated_rtsp_url(host, port, user, &pass, &raw_url);
        self.log_store.log("INFO", "CameraManager", &format!("Testando conexão com {}", full_url));
        
        probe_rtsp_stream(&full_url).await
    }

    pub async fn test_existing_camera_connection(&self, camera_id: &str) -> Result<CameraConnectionTestResult, String> {
        let cam = self.db.get_camera_by_id(camera_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Camera not found".to_string())?;

        let password = self.db.get_camera_decrypted_password(camera_id)?
            .unwrap_or_default();

        let full_url = build_authenticated_rtsp_url(&cam.host, cam.rtsp_port, &cam.username, &password, &cam.rtsp_url);
        self.log_store.log("INFO", "CameraManager", &format!("Testando conexão para câmera cadastrada {}", cam.name));
        
        Ok(probe_rtsp_stream(&full_url).await)
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
