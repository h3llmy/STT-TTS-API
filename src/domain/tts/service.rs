use crate::core::config::Config;
use sherpa_rs::OnnxConfig;
use sherpa_rs::tts::{KokoroTts, KokoroTtsConfig};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct TtsService {
    tts: Mutex<KokoroTts>,
}

impl TtsService {
    pub fn new(config: &Config) -> Self {
        let provider = if config.use_gpu {
            if cfg!(target_os = "macos") {
                "coreml".to_string()
            } else {
                "cuda".to_string()
            }
        } else {
            "cpu".to_string()
        };

        let tts_config = KokoroTtsConfig {
            model: config.tts_model.clone(),
            voices: config.tts_voices.clone(),
            tokens: config.tts_tokens.clone(),
            data_dir: config.tts_data_dir.clone(),
            onnx_config: OnnxConfig {
                provider,
                num_threads: config.threads,
                ..Default::default()
            },
            ..Default::default()
        };

        let tts = KokoroTts::new(tts_config);

        Self {
            tts: Mutex::new(tts),
        }
    }

    #[allow(dead_code)]
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

    pub fn generate_stream(
        self: Arc<Self>,
        text: &str,
        sid: i32,
        speed: f32,
    ) -> mpsc::Receiver<Vec<f32>> {
        let (tx, rx) = mpsc::channel(50);
        let text = text.to_string();
        let service = self.clone();

        tokio::task::spawn_blocking(move || {
            let mut current = String::new();
            let mut word_count = 0;
            let mut is_first_chunk = true;

            for c in text.chars() {
                current.push(c);
                if c == ' ' {
                    word_count += 1;
                }

                // Sensitivity settings
                let hard_boundary = c == '.' || c == '!' || c == '?' || c == '\n';
                let soft_boundary = c == ',' || c == ';' || c == ':' || c == '(' || c == ')';

                // Latency optimization:
                // 1. Send first chunk very aggressively (e.g. after first comma or 5 words)
                // 2. Otherwise split at hard boundaries or long soft-bounded segments
                let should_split = if is_first_chunk {
                    hard_boundary || (soft_boundary && current.len() > 20) || word_count >= 6
                } else {
                    hard_boundary || (soft_boundary && current.len() > 40) || word_count >= 15
                };

                if should_split {
                    let segment = current.trim().to_string();
                    if !segment.is_empty() {
                        let mut tts = service.tts.lock().unwrap();
                        match tts.create(&segment, sid, speed) {
                            Ok(audio) => {
                                if tx.blocking_send(audio.samples).is_err() {
                                    return;
                                }
                                is_first_chunk = false;
                                current.clear();
                                word_count = 0;
                            }
                            Err(e) => {
                                eprintln!("Streaming TTS Error: {}", e);
                                // If it fails, we keep 'current' to try merging with next part
                            }
                        }
                    }
                }
            }

            // Final segment
            if !current.trim().is_empty() {
                let segment = current.trim().to_string();
                if let Ok(mut tts) = service.tts.lock() {
                    if let Ok(audio) = tts.create(&segment, sid, speed) {
                        let _ = tx.blocking_send(audio.samples);
                    }
                }
            }
        });

        rx
    }
}
