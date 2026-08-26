use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

use crate::video::engine::VideoEngineManager;

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
    let rx = match manager.get_frame_receiver(&camera_id).await {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Camera stream not found").into_response(),
    };

    let stream = BroadcastStream::new(rx).filter_map(|res| {
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

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "multipart/x-mixed-replace; boundary=frame",
        )
        .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate")
        .header(header::PRAGMA, "no-cache")
        .header(header::EXPIRES, "0")
        .body(axum::body::Body::from_stream(stream))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Stream error").into_response())
}
