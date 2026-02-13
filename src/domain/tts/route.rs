use crate::domain::tts::handler::tts_handler;
use axum::{Router, routing::get};

pub fn route() -> Router {
    Router::new().route("/tts", get(tts_handler))
}
