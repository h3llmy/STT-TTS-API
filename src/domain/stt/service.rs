use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::core::config::Config;

pub enum TranscriptionResult {
    Partial(String),
    Final(String),
}

pub struct StreamProcessor {
    audio_buffer: Vec<f32>,
    last_processed_len: usize,
    last_transcribed_text: String,
}

impl StreamProcessor {
    pub fn new() -> Self {
        Self {
            audio_buffer: Vec::new(),
            last_processed_len: 0,
            last_transcribed_text: String::new(),
        }
    }

    pub fn process_audio(
        &mut self,
        bytes: &[u8],
        transcriber: &WhisperTranscriber,
    ) -> Vec<TranscriptionResult> {
        let mut results = Vec::new();

        let samples: Vec<f32> = bytes
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
            .collect();

        self.audio_buffer.extend(samples);

        // Periodic transcription for "real-time" feel (every ~500ms of new audio)
        if self.audio_buffer.len() - self.last_processed_len >= 8000 {
            let recent_samples = if self.audio_buffer.len() > 4000 {
                &self.audio_buffer[self.audio_buffer.len() - 4000..]
            } else {
                &self.audio_buffer[..]
            };

            if !self.is_silence_internal(recent_samples) {
                if let Some(text) = transcriber.transcribe(&self.audio_buffer) {
                    if text != self.last_transcribed_text {
                        results.push(TranscriptionResult::Partial(text.clone()));
                        self.last_transcribed_text = text;
                    }
                }
            }
            self.last_processed_len = self.audio_buffer.len();
        }

        // Finalization: If the last 1 second is silent, conclude the sentence.
        if self.audio_buffer.len() >= 16_000 {
            let last_1s = &self.audio_buffer[self.audio_buffer.len() - 16_000..];
            if self.is_silence_internal(last_1s) {
                if !self.last_transcribed_text.is_empty() {
                    results.push(TranscriptionResult::Final(
                        self.last_transcribed_text.clone(),
                    ));
                }

                self.audio_buffer.clear();
                self.last_processed_len = 0;
                self.last_transcribed_text.clear();
            }
        }

        // Safety limit to prevent buffer growing too large (e.g., 60s)
        if self.audio_buffer.len() > 16_000 * 60 {
            self.audio_buffer.drain(0..16_000 * 30);
            self.last_processed_len = self.audio_buffer.len();
        }

        results
    }

    fn is_silence_internal(&self, audio: &[f32]) -> bool {
        if audio.is_empty() {
            return true;
        }
        let energy: f32 = audio.iter().map(|s| s.abs()).sum::<f32>() / audio.len() as f32;
        energy < 0.012
    }
}

pub struct WhisperTranscriber {
    context: WhisperContext,
    config: Config,
}

impl WhisperTranscriber {
    pub fn new(config: &Config) -> Self {
        println!("Loading Whisper model...");
        let model_path = format!("models/{}", config.model);
        let mut whisper_params = WhisperContextParameters::default();
        whisper_params.use_gpu = config.use_gpu;
        whisper_params.gpu_device = config.gpu_device;

        let ctx = WhisperContext::new_with_params(&model_path, whisper_params)
            .expect("failed to load model");

        println!("Whisper model loaded.");
        Self {
            context: ctx,
            config: config.clone(),
        }
    }

    fn is_silence(&self, audio: &[f32]) -> bool {
        if audio.is_empty() {
            return true;
        }

        let energy: f32 = audio.iter().map(|s| s.abs()).sum::<f32>() / audio.len() as f32;

        energy < 0.015
    }

    pub fn transcribe(&self, audio: &[f32]) -> Option<String> {
        if self.is_silence(audio) {
            return None;
        }

        let mut state = self
            .context
            .create_state()
            .expect("failed to create whisper state");

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_n_threads(self.config.threads);
        params.set_translate(false);
        params.set_language(Some("auto"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        if state.full(params, audio).is_err() {
            return None;
        }

        let mut text = String::new();
        let n = state.full_n_segments();

        for i in 0..n {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(seg_text) = segment.to_str() {
                    text.push_str(seg_text);
                }
            }
        }

        let text = text.trim().to_string();

        if text.is_empty() { None } else { Some(text) }
    }
}
