use axum::{Router, routing::get};

use crate::domain::stt::handler::{index, stt_ws};

pub fn route() -> Router {
    Router::new()
        .route("/stt", get(stt_ws))
        .route("/", get(index))
}
