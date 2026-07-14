use crate::settings::Settings;
use axum::{extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State}, response::Response, routing::get, Router};
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{services::{ServeDir, ServeFile}, trace::TraceLayer};

type SharedSettings = Arc<RwLock<Settings>>;

async fn websocket(ws: WebSocketUpgrade, State(settings): State<SharedSettings>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, settings))
}
async fn handle_socket(mut socket: WebSocket, settings: SharedSettings) {
    let initial = serde_json::to_string(&*settings.read().await).unwrap_or_else(|_| "{}".into());
    if socket.send(Message::Text(initial.into())).await.is_err() { return; }
    while let Some(Ok(message)) = socket.next().await {
        let Message::Text(text) = message else { continue };
        match serde_json::from_str::<Settings>(&text) {
            Ok(updated) => {
                if let Err(error) = updated.save("settings.json").await {
                    tracing::warn!(%error, "could not save panel settings");
                    continue;
                }
                *settings.write().await = updated;
            }
            Err(error) => tracing::warn!(%error, "panel sent invalid settings"),
        }
    }
}

pub async fn run(settings: SharedSettings) -> anyhow::Result<()> {
    let static_files = ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"));
    let app = Router::new().route("/ws", get(websocket)).fallback_service(static_files)
        .layer(TraceLayer::new_for_http()).with_state(settings);
    let address = SocketAddr::from(([127, 0, 0, 1], 8999));
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "settings panel available");
    axum::serve(listener, app).await?;
    Ok(())
}
