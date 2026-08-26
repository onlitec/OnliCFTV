use std::time::Instant;
use tokio::process::Command;
use serde_json::Value;
use crate::camera::model::CameraConnectionTestResult;
use crate::logging::logger::sanitize_credentials;

pub async fn probe_rtsp_stream(full_rtsp_url: &str) -> CameraConnectionTestResult {
    let start_time = Instant::now();
    
    // Call ffprobe with TCP transport and 5 second timeout
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            "-show_format",
            "-rtsp_transport", "tcp",
            "-timeout", "5000000",
            full_rtsp_url,
        ])
        .output()
        .await;

    let latency = start_time.elapsed().as_millis() as u64;

    match output {
        Ok(res) if res.status.success() => {
            let stdout_str = String::from_utf8_lossy(&res.stdout);
            match serde_json::from_str::<Value>(&stdout_str) {
                Ok(json) => {
                    let mut codec = None;
                    let mut resolution = None;
                    let mut fps = None;

                    if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
                        for stream in streams {
                            if stream.get("codec_type").and_then(|t| t.as_str()) == Some("video") {
                                codec = stream.get("codec_name").and_then(|c| c.as_str()).map(|s| s.to_uppercase());
                                let width = stream.get("width").and_then(|w| w.as_u64());
                                let height = stream.get("height").and_then(|h| h.as_u64());
                                if let (Some(w), Some(h)) = (width, height) {
                                    resolution = Some(format!("{}x{}", w, h));
                                }

                                if let Some(rate_str) = stream.get("r_frame_rate").and_then(|r| r.as_str()) {
                                    if let Some((num, den)) = rate_str.split_once('/') {
                                        if let (Ok(n), Ok(d)) = (num.parse::<f32>(), den.parse::<f32>()) {
                                            if d > 0.0 {
                                                fps = Some((n / d * 10.0).round() / 10.0);
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }

                    let bitrate = json.get("format")
                        .and_then(|f| f.get("bit_rate"))
                        .and_then(|b| b.as_str())
                        .map(|b| {
                            if let Ok(bits) = b.parse::<u64>() {
                                format!("{:.1} kbps", bits as f64 / 1000.0)
                            } else {
                                b.to_string()
                            }
                        });

                    CameraConnectionTestResult {
                        success: true,
                        message: "Conexão RTSP estabelecida com sucesso".to_string(),
                        codec,
                        resolution,
                        fps,
                        bitrate,
                        latency_ms: Some(latency),
                    }
                }
                Err(e) => CameraConnectionTestResult {
                    success: false,
                    message: format!("Erro ao analisar resposta do stream: {}", e),
                    codec: None,
                    resolution: None,
                    fps: None,
                    bitrate: None,
                    latency_ms: Some(latency),
                }
            }
        }
        Ok(res) => {
            let stderr_str = String::from_utf8_lossy(&res.stderr);
            let sanitized_err = sanitize_credentials(&stderr_str);
            let msg = if sanitized_err.contains("401") || sanitized_err.contains("Unauthorized") {
                "Falha na autenticação RTSP (Usuário ou senha incorretos)".to_string()
            } else if sanitized_err.contains("Connection refused") {
                "Conexão recusada (Verifique se o IP e a porta RTSP 554 estão acessíveis)".to_string()
            } else if sanitized_err.contains("timed out") || sanitized_err.contains("Operation not permitted") {
                "Tempo limite de conexão esgotado (Timeout)".to_string()
            } else if sanitized_err.is_empty() {
                "Falha ao conectar no stream RTSP da câmera".to_string()
            } else {
                sanitized_err
            };

            CameraConnectionTestResult {
                success: false,
                message: msg,
                codec: None,
                resolution: None,
                fps: None,
                bitrate: None,
                latency_ms: Some(latency),
            }
        }
        Err(e) => CameraConnectionTestResult {
            success: false,
            message: format!("Falha ao executar ffprobe: {}", e),
            codec: None,
            resolution: None,
            fps: None,
            bitrate: None,
            latency_ms: Some(latency),
        }
    }
}
