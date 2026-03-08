use crate::shared::app_state::AppState;
use axum::{
    extract::{Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;

pub struct TtsController;

#[derive(Deserialize)]
pub struct TtsQuery {
    text: String,
    #[serde(default = "default_sid")]
    sid: i32,
    #[serde(default = "default_speed")]
    speed: f32,
}

fn default_sid() -> i32 {
    0
}
fn default_speed() -> f32 {
    1.0
}

impl TtsController {
    pub async fn generate(
        State(state): State<Arc<AppState>>,
        Query(query): Query<TtsQuery>,
    ) -> impl IntoResponse {
        if query.text.is_empty() {
            return (StatusCode::BAD_REQUEST, "Text is required").into_response();
        }

        let samples = state.tts.generate(&query.text, query.sid, query.speed);

        // Convert f32 samples to i16 for broader compatibility and smaller size
        let i16_samples: Vec<i16> = samples
            .into_iter()
            .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        let mut bytes = Vec::with_capacity(i16_samples.len() * 2);
        for sample in i16_samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        (
            [
                (
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static("audio/pcm"),
                ),
                (
                    header::HeaderName::from_static("x-sample-rate"),
                    header::HeaderValue::from_static("24000"),
                ),
            ],
            bytes,
        )
            .into_response()
    }
}
