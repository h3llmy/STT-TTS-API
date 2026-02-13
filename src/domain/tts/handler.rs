use crate::domain::tts::service::synthesize;
use axum::{
    extract::Query,
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TtsQuery {
    pub text: String,
    pub voice: Option<String>,
}

pub async fn tts_handler(Query(query): Query<TtsQuery>) -> impl IntoResponse {
    if query.text.is_empty() {
        return (StatusCode::BAD_REQUEST, "Text is required").into_response();
    }

    let audio_data = synthesize(&query.text, query.voice.as_deref());

    if audio_data.is_empty() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate audio",
        )
            .into_response();
    }

    (
        [
            (header::CONTENT_TYPE, "audio/wav"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"speech.wav\"",
            ),
        ],
        audio_data,
    )
        .into_response()
}
