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
    let mut last_processed_len = 0;
    let mut last_transcribed_text = String::new();

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Binary(bytes) => {
                let samples: Vec<f32> = bytes
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
                    .collect();

                audio_buffer.extend(samples);

                // Periodic transcription for "real-time" feel (every ~500ms of new audio)
                if audio_buffer.len() - last_processed_len >= 8000 {
                    // Only transcribe if the *recent* part of the buffer isn't total silence
                    let recent_samples = if audio_buffer.len() > 4000 {
                        &audio_buffer[audio_buffer.len() - 4000..]
                    } else {
                        &audio_buffer[..]
                    };

                    if !is_silence_internal(recent_samples) {
                        if let Some(text) = transcribe(&audio_buffer) {
                            if text != last_transcribed_text {
                                println!("Partial: {}", text);
                                let payload = json!({
                                    "type": "partial",
                                    "text": text
                                })
                                .to_string();
                                let _ = socket.send(Message::Text(payload.into())).await;
                                last_transcribed_text = text;
                            }
                        }
                    }
                    last_processed_len = audio_buffer.len();
                }

                // Finalization: If the last 1 second is silent, conclude the sentence.
                if audio_buffer.len() >= 16_000 {
                    let last_1s = &audio_buffer[audio_buffer.len() - 16_000..];
                    if is_silence_internal(last_1s) {
                        // If we had any meaningful transcription before this silence, send final
                        if !last_transcribed_text.is_empty() {
                            println!("Finalizing: {}", last_transcribed_text);
                            let payload = json!({
                                "type": "final",
                                "text": last_transcribed_text.clone()
                            })
                            .to_string();
                            let _ = socket.send(Message::Text(payload.into())).await;
                        }

                        // Clear state for next sentence
                        audio_buffer.clear();
                        last_processed_len = 0;
                        last_transcribed_text.clear();
                    }
                }

                // Safety limit
                if audio_buffer.len() > 16_000 * 60 {
                    audio_buffer.drain(0..16_000 * 30);
                    last_processed_len = audio_buffer.len();
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

fn is_silence_internal(audio: &[f32]) -> bool {
    if audio.is_empty() {
        return true;
    }
    let energy: f32 = audio.iter().map(|s| s.abs()).sum::<f32>() / audio.len() as f32;
    // Adjusted threshold to be more reliable in real-world environments
    energy < 0.012
}

pub async fn index() -> impl IntoResponse {
    let html = fs::read_to_string("views/index.html").expect("Failed to read index.html");
    Html(html)
}
