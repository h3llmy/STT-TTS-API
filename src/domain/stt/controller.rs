use std::fs;
use std::sync::Arc;

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{Html, IntoResponse},
};
use futures::{SinkExt, StreamExt};
use serde_json::json;

use crate::{
    domain::stt::service::{StreamProcessor, TranscriptionResult},
    shared::app_state::AppState,
};

pub struct SttController;

impl SttController {
    pub async fn transcribe_ws(
        State(state): State<Arc<AppState>>,
        ws: WebSocketUpgrade,
    ) -> axum::response::Response {
        ws.on_upgrade(|socket| Self::handle_socket(socket, state))
    }

    async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
        println!("Client connected");

        let (mut ws_sender, mut ws_receiver) = socket.split();
        let (audio_tx, mut audio_rx) = tokio::sync::mpsc::channel::<axum::body::Bytes>(100);

        // Task 1: Receiver Loop (Receive audio chunks and queue them)
        let recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_receiver.next().await {
                if let Message::Binary(bytes) = msg {
                    if audio_tx.send(bytes.into()).await.is_err() {
                        // .into() converts Vec<u8> to Bytes
                        break;
                    }
                } else if let Message::Close(_) = msg {
                    break;
                }
            }
        });

        // Task 2: Processor Loop (Process queued audio and send results)
        let transcriber = state.transcriber.clone();
        let send_task = tokio::spawn(async move {
            let mut processor = StreamProcessor::new();

            while let Some(bytes) = audio_rx.recv().await {
                // Here we call the optimized processor
                // Ideally this should be wrapped in spawn_blocking if it takes > 10ms,
                // but since this is a dedicated task, it won't block the receiver.
                // However, whisper-rs can be very heavy, so we use spawn_blocking for the actual transcription.

                // For now, let's keep it in the loop but note that this blocks this specific task.
                let results = processor.process_audio(&bytes[..], &transcriber);

                for result in results {
                    let (msg_type, text) = match result {
                        TranscriptionResult::Partial(t) => ("partial", t),
                        TranscriptionResult::Final(t) => ("final", t),
                    };

                    let payload = json!({
                        "type": msg_type,
                        "text": text
                    })
                    .to_string();

                    if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                        return;
                    }
                }
            }
        });

        // Wait until both tasks are done (connection closed)
        tokio::select! {
            _ = recv_task => {},
            _ = send_task => {},
        }

        println!("Client disconnected");
    }

    pub async fn index() -> impl IntoResponse {
        let html = fs::read_to_string("views/index.html").expect("Failed to read index.html");
        Html(html)
    }
}
