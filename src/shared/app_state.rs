use std::sync::Arc;

use crate::domain::stt::service::WhisperTranscriber;

pub struct AppState {
    pub transcriber: Arc<WhisperTranscriber>,
}
