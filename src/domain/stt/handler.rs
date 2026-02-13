use std::fs;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
};
use futures::StreamExt;
use serde_json::json;

use crate::domain::stt::service::StreamingTranscriber;

pub async fn stt_ws(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    println!("Client connected");

    // Create a persistent transcriber for this connection
    let mut transcriber = StreamingTranscriber::new();

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Binary(bytes) => {
                // Convert bytes to f32 samples
                let samples: Vec<f32> = bytes
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
                    .collect();

                // Add audio to the rolling window
                transcriber.add_audio(&samples);

                // Check if we should transcribe
                if transcriber.should_transcribe() {
                    let result = transcriber.transcribe_incremental();

                    // Send partial results if we have new content
                    if !result.partial_text.is_empty() {
                        let payload = json!({
                            "type": "partial",
                            "text": result.partial_text,
                            "full_text": result.full_text
                        });

                        if socket
                            .send(Message::Text(payload.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }

            Message::Text(text) => {
                // Handle control messages
                if text == "finalize" {
                    let result = transcriber.finalize();

                    if result.has_content() {
                        let payload = json!({
                            "type": "final",
                            "text": result.full_text
                        });

                        if socket
                            .send(Message::Text(payload.to_string().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }

                    transcriber.reset();
                } else if text == "reset" {
                    transcriber.reset();

                    let payload = json!({
                        "type": "reset",
                        "text": ""
                    });

                    let _ = socket.send(Message::Text(payload.to_string().into())).await;
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
