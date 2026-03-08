use std::sync::Arc;

use crate::{domain::stt::controller::SttController, shared::app_state::AppState};
use axum::{Router, routing::get};

pub fn stt_route() -> Router<Arc<AppState>> {
    Router::new().nest(
        "/stt",
        Router::new()
            .route("/", get(SttController::index))
            .route("/transcribe", get(SttController::transcribe_ws)),
    )
}
