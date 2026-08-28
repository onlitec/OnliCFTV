use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

use crate::video::engine::VideoEngineManager;

/// Espera máxima pelo primeiro frame de uma sessão nova. Precisa acomodar o pior
/// caso de H.265+ com Smart Codec, onde o intervalo entre I-Frames chega a 4s e o
/// FFmpeg ainda gasta ~2s de `analyzeduration` para capturar VPS/SPS/PPS.
const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn start_stream_server(manager: VideoEngineManager, port: u16) {
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/stream/{camera_id}", get(mjpeg_stream_handler))
        .route("/snapshot/{camera_id}", get(snapshot_handler))
        .layer(CorsLayer::permissive())
        .with_state(manager);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    
    tokio::spawn(async move {
        if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
            let _ = axum::serve(listener, app).await;
        }
    });
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OnliView Video Stream Server OK")
}

async fn snapshot_handler(
    Path(camera_id): Path<String>,
    State(manager): State<VideoEngineManager>,
) -> Response {
    if let Some(frame) = manager.get_latest_frame(&camera_id).await {
        ([(header::CONTENT_TYPE, "image/jpeg")], (*frame).clone()).into_response()
    } else {
        (StatusCode::NOT_FOUND, "No frame available").into_response()
    }
}

async fn mjpeg_stream_handler(
    Path(camera_id): Path<String>,
    State(manager): State<VideoEngineManager>,
) -> Response {
    let mut rx = match manager.get_frame_receiver(&camera_id).await {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Camera stream not found").into_response(),
    };

    // Numa sessão já ativa o último frame em cache abre a conexão instantaneamente.
    // Numa sessão recém-criada o cache está vazio: responder 200 com um multipart que
    // nunca emite nada deixaria a tag <img> preta para sempre — um 200 não dispara
    // `onError`, então o retry automático do frontend nunca seria acionado. Esperamos
    // então o primeiro frame e devolvemos 503 se ele não vier, deixando o cliente
    // tentar de novo enquanto o motor reconecta.
    let initial_frame = match manager.get_latest_frame(&camera_id).await {
        Some(frame) => frame,
        None => match tokio::time::timeout(FIRST_FRAME_TIMEOUT, rx.recv()).await {
            Ok(Ok(frame)) => frame,
            _ => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Stream sem vídeo disponível",
                )
                    .into_response()
            }
        },
    };

    let initial_stream = tokio_stream::iter(std::iter::once(initial_frame).map(|frame| {
        let header = format!(
            "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
            frame.len()
        );
        let mut chunk = header.into_bytes();
        chunk.extend_from_slice(&frame);
        chunk.extend_from_slice(b"\r\n");
        Ok::<_, std::io::Error>(chunk)
    }));

    let broadcast_stream = BroadcastStream::new(rx).filter_map(|res| {
        match res {
            Ok(frame) => {
                let header = format!(
                    "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                    frame.len()
                );
                let mut chunk = header.into_bytes();
                chunk.extend_from_slice(&frame);
                chunk.extend_from_slice(b"\r\n");
                Some(Ok::<_, std::io::Error>(chunk))
            }
            Err(_) => None,
        }
    });

    let combined_stream = initial_stream.chain(broadcast_stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "multipart/x-mixed-replace; boundary=frame",
        )
        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
        .header(header::PRAGMA, "no-cache")
        .header(header::EXPIRES, "0")
        .body(axum::body::Body::from_stream(combined_stream))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Stream error").into_response())
}
