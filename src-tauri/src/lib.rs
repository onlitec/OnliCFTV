pub mod camera;
pub mod rtsp;
pub mod onvif;
pub mod discovery;
pub mod video;
pub mod database;
pub mod configuration;
pub mod logging;

use tauri::State;

use crate::camera::model::*;
use crate::camera::manager::CameraManager;
use crate::camera::recording::RecordingCheckResult;
use crate::discovery::NetworkInterfaceInfo;
use crate::video::engine::{VideoEngineManager, CameraStreamStatus};
use crate::logging::logger::{LogStore, LogEntry};
use crate::configuration::config::AppConfig;
use crate::database::Database;

pub struct AppState {
    pub config: AppConfig,
    pub camera_manager: CameraManager,
    pub video_engine: VideoEngineManager,
    pub log_store: LogStore,
}

#[tauri::command]
async fn get_network_interfaces(state: State<'_, AppState>) -> Result<Vec<NetworkInterfaceInfo>, String> {
    Ok(state.camera_manager.get_network_interfaces())
}

#[tauri::command]
async fn get_cameras(state: State<'_, AppState>) -> Result<Vec<Camera>, String> {
    state.camera_manager.get_cameras()
}

#[tauri::command]
async fn get_camera(id: String, state: State<'_, AppState>) -> Result<Option<Camera>, String> {
    state.camera_manager.get_camera_by_id(&id)
}

#[tauri::command]
async fn create_camera(input: CreateCameraInput, state: State<'_, AppState>) -> Result<Camera, String> {
    state.camera_manager.create_camera(input).await
}

#[tauri::command]
async fn create_cameras_batch(input: BatchCreateCamerasInput, state: State<'_, AppState>) -> Result<Vec<Camera>, String> {
    state.camera_manager.create_cameras_batch(input).await
}

#[tauri::command]
async fn discover_devices(interface_name: Option<String>, state: State<'_, AppState>) -> Result<Vec<DiscoveredDevice>, String> {
    state.camera_manager.discover_devices(interface_name).await
}

#[tauri::command]
async fn update_camera(input: UpdateCameraInput, state: State<'_, AppState>) -> Result<Camera, String> {
    state.camera_manager.update_camera(input)
}

#[tauri::command]
async fn delete_camera(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.delete_camera(&id).await
}

#[tauri::command]
async fn delete_cameras_batch(ids: Vec<String>, state: State<'_, AppState>) -> Result<usize, String> {
    state.camera_manager.delete_cameras_batch(ids).await
}

#[tauri::command]
async fn delete_all_cameras(state: State<'_, AppState>) -> Result<usize, String> {
    state.camera_manager.delete_all_cameras().await
}

#[tauri::command]
async fn test_camera_connection(input: CreateCameraInput, state: State<'_, AppState>) -> Result<CameraConnectionTestResult, String> {
    Ok(state.camera_manager.test_connection(input).await)
}

#[tauri::command]
async fn test_existing_camera(id: String, state: State<'_, AppState>) -> Result<CameraConnectionTestResult, String> {
    state.camera_manager.test_existing_camera_connection(&id).await
}

#[tauri::command]
async fn start_stream(camera_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.start_camera_stream(&camera_id).await
}

#[tauri::command]
async fn stop_stream(camera_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.stop_camera_stream(&camera_id).await
}

#[tauri::command]
async fn get_stream_status(camera_id: String, state: State<'_, AppState>) -> Result<Option<CameraStreamStatus>, String> {
    Ok(state.video_engine.get_status(&camera_id).await)
}

#[tauri::command]
async fn get_all_stream_statuses(state: State<'_, AppState>) -> Result<Vec<CameraStreamStatus>, String> {
    Ok(state.video_engine.get_all_statuses().await)
}

#[tauri::command]
async fn get_logs(state: State<'_, AppState>) -> Result<Vec<LogEntry>, String> {
    Ok(state.log_store.get_logs())
}

#[tauri::command]
async fn clear_logs(state: State<'_, AppState>) -> Result<(), String> {
    state.log_store.clear();
    Ok(())
}

#[tauri::command]
async fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.clone())
}

#[tauri::command]
async fn quick_view_connect(input: QuickViewConnectInput, state: State<'_, AppState>) -> Result<QuickViewSessionInfo, String> {
    state.camera_manager.quick_view_connect(input).await
}

#[tauri::command]
async fn quick_view_disconnect(ip: String, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.quick_view_disconnect(&ip).await
}

#[tauri::command]
async fn quick_view_set_device_name(input: QuickViewSetDeviceNameInput, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.quick_view_set_device_name(input).await
}

#[tauri::command]
async fn quick_view_set_osd(input: QuickViewSetOsdInput, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.quick_view_set_osd(input).await
}

#[tauri::command]
async fn start_device_preview(input: QuickViewConnectInput, state: State<'_, AppState>) -> Result<String, String> {
    state.camera_manager.start_device_preview(input).await
}

#[tauri::command]
async fn stop_device_preview(ip: String, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.stop_device_preview(&ip).await
}

#[tauri::command]
async fn get_device_credentials(ip: String, mac: Option<String>, state: State<'_, AppState>) -> Result<Option<CachedDeviceCredentials>, String> {
    state.camera_manager.get_cached_credentials(&ip, mac.as_deref())
}

#[tauri::command]
async fn forget_device_credentials(ip: String, state: State<'_, AppState>) -> Result<(), String> {
    state.camera_manager.forget_device_credentials(&ip)
}

#[tauri::command]
async fn check_recordings(
    period_start: Option<String>,
    period_end: Option<String>,
    nvr_ids: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<RecordingCheckResult, String> {
    state
        .camera_manager
        .check_recordings(period_start, period_end, nvr_ids)
        .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::default();
    let log_store = LogStore::new(1000);
    
    log_store.log("INFO", "App", "Inicializando OnliView VMS Engine");

    let db = match Database::new(&config.database_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open database at {:?}: {}", config.database_path, e);
            std::process::exit(1);
        }
    };

    let video_engine = VideoEngineManager::new(log_store.clone(), config.video_server_port);
    let camera_manager = CameraManager::new(db.clone(), video_engine.clone(), log_store.clone());

    // Start background MJPEG stream HTTP server on local port
    let video_server_engine = video_engine.clone();
    let port = config.video_server_port;
    tokio::spawn(async move {
        crate::video::stream_server::start_stream_server(video_server_engine, port).await;
    });

    let app_state = AppState {
        config,
        camera_manager,
        video_engine,
        log_store,
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_network_interfaces,
            get_cameras,
            get_camera,
            create_camera,
            create_cameras_batch,
            discover_devices,
            update_camera,
            delete_camera,
            delete_cameras_batch,
            delete_all_cameras,
            test_camera_connection,
            test_existing_camera,
            start_stream,
            stop_stream,
            get_stream_status,
            get_all_stream_statuses,
            get_logs,
            clear_logs,
            get_app_config,
            quick_view_connect,
            quick_view_disconnect,
            quick_view_set_device_name,
            quick_view_set_osd,
            start_device_preview,
            stop_device_preview,
            get_device_credentials,
            forget_device_credentials,
            check_recordings
        ])
        .run(tauri::generate_context!())
        .expect("error while running OnliView application");
}
