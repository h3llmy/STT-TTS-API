use crate::core::config::Config;
use crate::domain::stt::service::WhisperTranscriber;
use crate::presentation::*;
use axum::Router;
use std::sync::Arc;

pub struct AppState {
    pub transcriber: Arc<WhisperTranscriber>,
}

pub async fn build_app(config: &Config) -> Router {
    dotenvy::dotenv().ok();

    let transcriber = Arc::new(WhisperTranscriber::new(&config));

    let shared_state = Arc::new(AppState { transcriber });

    let app = Router::new()
        .merge(stt_route())
        .with_state(shared_state.clone());

    app
}
