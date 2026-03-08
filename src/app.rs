use crate::{
    core::config::Config, domain::stt::service::WhisperTranscriber, presentation::*,
    shared::app_state::AppState,
};
use axum::Router;
use std::sync::Arc;

pub async fn build_app(config: &Config) -> Router {
    dotenvy::dotenv().ok();

    let transcriber = Arc::new(WhisperTranscriber::new(&config));

    let shared_state = Arc::new(AppState { transcriber });

    let app = Router::new()
        .merge(stt_route())
        .with_state(shared_state.clone());

    app
}
