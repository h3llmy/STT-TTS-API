use std::fs;
use std::sync::Arc;

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{Html, IntoResponse},
};
use futures::StreamExt;
use serde_json::json;

use crate::app::AppState;
use crate::domain::stt::service::{StreamProcessor, TranscriptionResult};

pub struct SttController;

impl SttController {
    pub async fn transcribe_ws(
        State(state): State<Arc<AppState>>,
        ws: WebSocketUpgrade,
    ) -> axum::response::Response {
        ws.on_upgrade(|socket| Self::handle_socket(socket, state))
    }

    async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
        println!("Client connected");

        let mut processor = StreamProcessor::new();

        while let Some(Ok(msg)) = socket.next().await {
            match msg {
                Message::Binary(bytes) => {
                    let results = processor.process_audio(&bytes, &state.transcriber);

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

                        if socket.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
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
}
