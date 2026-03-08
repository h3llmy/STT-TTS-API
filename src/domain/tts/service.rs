use crate::core::config::Config;
use sherpa_rs::tts::{VitsTts, VitsTtsConfig};
use std::sync::Mutex;

pub struct TtsService {
    // VitsTts needs &mut self for create, so we wrap it in Mutex for thread safety
    tts: Mutex<VitsTts>,
}

impl TtsService {
    pub fn new(config: &Config) -> Self {
        let tts_config = VitsTtsConfig {
            model: config.tts_model.clone(),
            lexicon: config.tts_voices.clone(), // Map voices to lexicon for VITS
            tokens: config.tts_tokens.clone(),
            data_dir: config.tts_data_dir.clone(),
            ..Default::default()
        };

        let tts = VitsTts::new(tts_config);

        Self {
            tts: Mutex::new(tts),
        }
    }

    pub fn generate(&self, text: &str, sid: i32, speed: f32) -> Vec<f32> {
        let mut tts = self.tts.lock().unwrap();
        match tts.create(text, sid, speed) {
            Ok(audio) => audio.samples,
            Err(e) => {
                eprintln!("TTS Error: {}", e);
                Vec::new()
            }
        }
    }
}
