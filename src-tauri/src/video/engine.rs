use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, watch, RwLock};
use tokio::process::Command;
use tokio::io::AsyncReadExt;
use serde::{Serialize, Deserialize};

use crate::logging::logger::LogStore;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StreamState {
    Online,
    Offline,
    Connecting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraStreamStatus {
    pub camera_id: String,
    pub state: StreamState,
    pub fps: f32,
    pub bitrate_kbps: f32,
    pub resolution: String,
    pub codec: String,
    pub reconnect_attempts: u32,
    pub last_frame_time: Option<String>,
    pub error_message: Option<String>,
    pub stream_url: String,
}

pub struct CameraSession {
    pub camera_id: String,
    pub rtsp_url: String,
    pub state: Arc<RwLock<CameraStreamStatus>>,
    pub frame_sender: broadcast::Sender<Arc<Vec<u8>>>,
    pub latest_frame: Arc<RwLock<Option<Arc<Vec<u8>>>>>,
    pub cancel_sender: watch::Sender<bool>,
}

#[derive(Clone)]
pub struct VideoEngineManager {
    sessions: Arc<RwLock<HashMap<String, Arc<CameraSession>>>>,
    log_store: LogStore,
    server_port: u16,
}

impl VideoEngineManager {
    pub fn new(log_store: LogStore, server_port: u16) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            log_store,
            server_port,
        }
    }

    pub fn server_port(&self) -> u16 {
        self.server_port
    }

    pub async fn connect(&self, camera_id: &str, rtsp_url: &str) -> Result<(), String> {
        self.stop(camera_id).await.ok();

        let initial_status = CameraStreamStatus {
            camera_id: camera_id.to_string(),
            state: StreamState::Connecting,
            fps: 0.0,
            bitrate_kbps: 0.0,
            resolution: "---".to_string(),
            codec: "H.264".to_string(),
            reconnect_attempts: 0,
            last_frame_time: None,
            error_message: None,
            stream_url: format!("http://127.0.0.1:{}/stream/{}", self.server_port, camera_id),
        };

        let (frame_tx, _) = broadcast::channel(16);
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let status_arc = Arc::new(RwLock::new(initial_status));
        let latest_frame_arc = Arc::new(RwLock::new(None));

        let session = Arc::new(CameraSession {
            camera_id: camera_id.to_string(),
            rtsp_url: rtsp_url.to_string(),
            state: status_arc.clone(),
            frame_sender: frame_tx.clone(),
            latest_frame: latest_frame_arc.clone(),
            cancel_sender: cancel_tx,
        });

        {
            let mut lock = self.sessions.write().await;
            lock.insert(camera_id.to_string(), session.clone());
        }

        self.log_store.log("INFO", "VideoEngine", &format!("Iniciando stream para câmera {}: {}", camera_id, rtsp_url));

        // Spawn background worker loop with auto-reconnection
        let cam_id = camera_id.to_string();
        let url = rtsp_url.to_string();
        let logger = self.log_store.clone();

        tokio::spawn(async move {
            run_camera_stream_worker(cam_id, url, status_arc, frame_tx, latest_frame_arc, cancel_rx, logger).await;
        });

        Ok(())
    }

    pub async fn stop(&self, camera_id: &str) -> Result<(), String> {
        let mut lock = self.sessions.write().await;
        if let Some(session) = lock.remove(camera_id) {
            let _ = session.cancel_sender.send(true);
            self.log_store.log("INFO", "VideoEngine", &format!("Stream interrompido para câmera {}", camera_id));
        }
        Ok(())
    }

    pub async fn get_status(&self, camera_id: &str) -> Option<CameraStreamStatus> {
        let lock = self.sessions.read().await;
        if let Some(session) = lock.get(camera_id) {
            let status = session.state.read().await;
            Some(status.clone())
        } else {
            None
        }
    }

    pub async fn get_all_statuses(&self) -> Vec<CameraStreamStatus> {
        let lock = self.sessions.read().await;
        let mut results = Vec::new();
        for session in lock.values() {
            let status = session.state.read().await;
            results.push(status.clone());
        }
        results
    }

    pub async fn get_frame_receiver(&self, camera_id: &str) -> Option<broadcast::Receiver<Arc<Vec<u8>>>> {
        let lock = self.sessions.read().await;
        lock.get(camera_id).map(|s| s.frame_sender.subscribe())
    }

    pub async fn get_latest_frame(&self, camera_id: &str) -> Option<Arc<Vec<u8>>> {
        let lock = self.sessions.read().await;
        if let Some(s) = lock.get(camera_id) {
            let frame = s.latest_frame.read().await;
            frame.clone()
        } else {
            None
        }
    }
}

async fn run_camera_stream_worker(
    camera_id: String,
    rtsp_url: String,
    status: Arc<RwLock<CameraStreamStatus>>,
    frame_tx: broadcast::Sender<Arc<Vec<u8>>>,
    latest_frame: Arc<RwLock<Option<Arc<Vec<u8>>>>>,
    mut cancel_rx: watch::Receiver<bool>,
    logger: LogStore,
) {
    let mut reconnect_count = 0;

    loop {
        if *cancel_rx.borrow() {
            break;
        }

        {
            let mut st = status.write().await;
            st.state = if reconnect_count == 0 { StreamState::Connecting } else { StreamState::Connecting };
            st.reconnect_attempts = reconnect_count;
        }

        logger.log("INFO", "VideoEngine", &format!("Tentando conexão RTSP ({}) para câmera {}", reconnect_count + 1, camera_id));

        // Spawn FFmpeg to extract MJPEG frames via stdout pipe with low-latency flags
        let mut child = match Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel", "error",
                "-rtsp_transport", "tcp",
                "-timeout", "5000000",
                "-fflags", "nobuffer+discardcorrupt",
                "-flags", "low_delay",
                "-i", &rtsp_url,
                "-an",
                "-c:v", "mjpeg",
                "-q:v", "5",
                "-r", "20",
                "-f", "image2pipe",
                "-vcodec", "mjpeg",
                "pipe:1",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Falha ao iniciar motor FFmpeg: {}", e);
                logger.log("ERROR", "VideoEngine", &err_msg);
                let mut st = status.write().await;
                st.state = StreamState::Error;
                st.error_message = Some(err_msg);
                tokio::time::sleep(Duration::from_secs(5)).await;
                reconnect_count += 1;
                continue;
            }
        };

        let mut stdout = match child.stdout.take() {
            Some(out) => out,
            None => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let mut buffer = Vec::with_capacity(65536);
        let mut temp_chunk = [0u8; 8192];
        let mut frame_count = 0usize;
        let mut bytes_count = 0usize;
        let mut fps_timer = Instant::now();
        let mut is_first_frame = true;

        loop {
            tokio::select! {
                _ = cancel_rx.changed() => {
                    if *cancel_rx.borrow() {
                        let _ = child.kill().await;
                        return;
                    }
                }
                read_res = stdout.read(&mut temp_chunk) => {
                    match read_res {
                        Ok(0) => {
                            // EOF
                            break;
                        }
                        Ok(n) => {
                            buffer.extend_from_slice(&temp_chunk[..n]);
                            bytes_count += n;

                            // Scan for JPEG boundaries: SOI (0xFF, 0xD8) to EOI (0xFF, 0xD9)
                            while let Some(soi_pos) = find_jpeg_soi(&buffer) {
                                if soi_pos > 0 {
                                    buffer.drain(0..soi_pos);
                                }
                                if let Some(eoi_pos) = find_jpeg_eoi(&buffer) {
                                    let frame_data = buffer.drain(0..=eoi_pos).collect::<Vec<u8>>();
                                    let frame_arc = Arc::new(frame_data);
                                    
                                    frame_count += 1;
                                    {
                                        let mut lf = latest_frame.write().await;
                                        *lf = Some(frame_arc.clone());
                                    }
                                    let _ = frame_tx.send(frame_arc);

                                    if is_first_frame {
                                        is_first_frame = false;
                                        reconnect_count = 0;
                                        let mut st = status.write().await;
                                        st.state = StreamState::Online;
                                        st.error_message = None;
                                        logger.log("INFO", "VideoEngine", &format!("Stream Online recebendo vídeo para câmera {}", camera_id));
                                    }

                                    // Compute FPS and Bitrate every 1s
                                    let elapsed = fps_timer.elapsed();
                                    if elapsed >= Duration::from_secs(1) {
                                        let secs = elapsed.as_secs_f32();
                                        let current_fps = (frame_count as f32 / secs * 10.0).round() / 10.0;
                                        let current_bitrate = (bytes_count as f32 * 8.0 / 1000.0 / secs * 10.0).round() / 10.0;
                                        
                                        let mut st = status.write().await;
                                        st.fps = current_fps;
                                        st.bitrate_kbps = current_bitrate;
                                        st.last_frame_time = Some(chrono::Utc::now().to_rfc3339());
                                        
                                        frame_count = 0;
                                        bytes_count = 0;
                                        fps_timer = Instant::now();
                                    }
                                } else {
                                    // Incomplete frame, wait for more chunks
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            logger.log("WARN", "VideoEngine", &format!("Erro na leitura do stream da câmera {}: {}", camera_id, e));
                            break;
                        }
                    }
                }
            }
        }

        let _ = child.kill().await;

        if *cancel_rx.borrow() {
            break;
        }

        // Connection dropped or failed, update state to Offline and auto-reconnect
        {
            let mut st = status.write().await;
            st.state = StreamState::Offline;
            st.fps = 0.0;
            st.bitrate_kbps = 0.0;
            st.error_message = Some("Conexão perdida. Tentando reconectar...".to_string());
        }
        reconnect_count += 1;
        logger.log("WARN", "VideoEngine", &format!("Câmera {} desconectada. Reconectando em 5 segundos (Tentativa {})...", camera_id, reconnect_count));

        // Sleep with cancellation support
        tokio::select! {
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
        }
    }
}

fn find_jpeg_soi(data: &[u8]) -> Option<usize> {
    if data.len() < 2 {
        return None;
    }
    data.windows(2).position(|w| w[0] == 0xFF && w[1] == 0xD8)
}

fn find_jpeg_eoi(data: &[u8]) -> Option<usize> {
    if data.len() < 2 {
        return None;
    }
    data.windows(2).position(|w| w[0] == 0xFF && w[1] == 0xD9).map(|pos| pos + 1)
}
