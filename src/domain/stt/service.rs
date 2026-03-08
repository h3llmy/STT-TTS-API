use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

use crate::core::config::Config;

pub enum TranscriptionResult {
    Partial(String),
    Final(String),
}

pub struct StreamProcessor {
    audio_buffer: Vec<f32>,
    last_processed_len: usize,
    last_transcribed_text: String,
    whisper_state: Option<WhisperState>,
    detected_language: Option<String>,
}

impl StreamProcessor {
    pub fn new() -> Self {
        Self {
            audio_buffer: Vec::with_capacity(16_000 * 30), // Pre-allocate 30s
            last_processed_len: 0,
            last_transcribed_text: String::new(),
            whisper_state: None,
            detected_language: None,
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

        // Lazy-initialize whisper state
        if self.whisper_state.is_none() {
            self.whisper_state = Some(
                transcriber
                    .context
                    .create_state()
                    .expect("Failed to create whisper state"),
            );
        }

        // Periodic transcription for "real-time" feel (every ~500ms of new audio)
        // Optimization: Use a sliding window of the last 15-30 seconds for partial results.
        // This is much faster than transcribing an unlimited buffer while maintaining context.
        if self.audio_buffer.len() - self.last_processed_len >= 8000 {
            let window_size = 16_000 * 30; // 30 seconds
            let start = if self.audio_buffer.len() > window_size {
                self.audio_buffer.len() - window_size
            } else {
                0
            };

            let audio_window = &self.audio_buffer[start..];

            if !self.is_silence_internal(audio_window) {
                if let Some(state) = self.whisper_state.as_mut() {
                    if let Some(text) = transcriber.transcribe_with_state(
                        state,
                        audio_window,
                        self.detected_language.as_deref(),
                    ) {
                        // After first transcription, if language was auto-detected, we can try to lock it
                        // Note: whisper_rs doesn't easily expose the detected language from the state without more calls,
                        // but we can assume auto-detection works better if we lock it later or just keep auto if fast enough.
                        // For now, let's keep it simple.

                        if text != self.last_transcribed_text {
                            results.push(TranscriptionResult::Partial(text.clone()));
                            self.last_transcribed_text = text;
                        }
                    }
                }
            }
            self.last_processed_len = self.audio_buffer.len();
        }

        // Finalization: If the last 1.2 seconds are silent, conclude the sentence.
        if self.audio_buffer.len() >= 16_000 + 3200 {
            let silence_check_len = 16_000 + 3200; // 1.2s
            let last_part = &self.audio_buffer[self.audio_buffer.len() - silence_check_len..];
            if self.is_silence_internal(last_part) {
                if !self.last_transcribed_text.is_empty() {
                    // One final transcription of the FULL buffer for maximum accuracy before clearing
                    if let Some(state) = self.whisper_state.as_mut() {
                        if let Some(text) = transcriber.transcribe_with_state(
                            state,
                            &self.audio_buffer,
                            self.detected_language.as_deref(),
                        ) {
                            results.push(TranscriptionResult::Final(text));
                        } else {
                            // Fallback to last partial if final failed
                            results.push(TranscriptionResult::Final(
                                self.last_transcribed_text.clone(),
                            ));
                        }
                    }
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
        energy < 0.015
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

    pub fn transcribe_with_state(
        &self,
        state: &mut WhisperState,
        audio: &[f32],
        language: Option<&str>,
    ) -> Option<String> {
        if self.is_silence(audio) {
            return None;
        }

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_n_threads(self.config.threads);
        params.set_translate(false);
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Speed up transcription by decreasing the max number of segments if possible
        // but default is usually fine.

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

    #[allow(dead_code)]
    pub fn transcribe(&self, audio: &[f32]) -> Option<String> {
        let mut state = self
            .context
            .create_state()
            .expect("failed to create whisper state");
        self.transcribe_with_state(&mut state, audio, None)
    }
}
