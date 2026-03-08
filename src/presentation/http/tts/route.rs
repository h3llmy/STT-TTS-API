use crate::presentation::http::tts::controller::TtsController;
use crate::shared::app_state::AppState;
use axum::{Router, routing::get};
use std::sync::Arc;

pub fn tts_route() -> Router<Arc<AppState>> {
    Router::new().nest(
        "/tts",
        Router::new().route("/generate", get(TtsController::generate)),
    )
}
