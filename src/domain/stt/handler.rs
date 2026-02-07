use std::fs;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
};
use futures::StreamExt;
use serde_json::json;

use crate::domain::stt::service::transcribe;

pub async fn stt_ws(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    println!("Client connected");

    let mut audio_buffer: Vec<f32> = Vec::new();

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Binary(bytes) => {
                let samples = bytes
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0);

                audio_buffer.extend(samples);

                if audio_buffer.len() >= 16_000 {
                    if let Some(text) = transcribe(&audio_buffer) {
                        let payload = json!({
                            "type": "final",
                            "text": text
                        })
                        .to_string();

                        if socket.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }

                    audio_buffer.clear();
                }
            }

            Message::Close(_) => {
                println!("Client disconnected");
                break;
            }

            _ => {}
        }
    }
}

pub async fn index() -> impl IntoResponse {
    let html = fs::read_to_string("views/index.html").expect("Failed to read index.html");
    Html(html)
}
