use std::sync::Arc;

use crate::domain::stt::service::WhisperTranscriber;
use crate::domain::tts::service::TtsService;

pub struct AppState {
    pub transcriber: Arc<WhisperTranscriber>,
    pub tts: Arc<TtsService>,
}
