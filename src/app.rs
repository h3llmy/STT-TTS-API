use crate::{
    core::config::Config, domain::stt::service::WhisperTranscriber, presentation::*,
    shared::app_state::AppState,
};
use axum::Router;
use std::sync::Arc;

fn depedencies_injection(config: &Config) -> Arc<AppState> {
    let transcriber = Arc::new(WhisperTranscriber::new(&config));

    Arc::new(AppState { transcriber })
}

pub async fn build_app(config: &Config) -> Router {
    dotenvy::dotenv().ok();

    let shared_state = depedencies_injection(config);

    let app = Router::new()
        .merge(stt_route())
        .with_state(shared_state.clone());

    app
}
