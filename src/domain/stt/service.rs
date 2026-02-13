use std::env;

use once_cell::sync::Lazy;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub static WHISPER: Lazy<WhisperContext> = Lazy::new(|| {
    println!("Loading Whisper model...");
    let model_path = format!("models/{}.bin", env::var("MODEL").expect("MODEL not set"));
    let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
        .expect("failed to load model");

    println!("Whisper model loaded.");
    ctx
});

// Configuration constants for real-time streaming
const SAMPLE_RATE: usize = 16_000;
const WINDOW_SIZE_SECONDS: usize = 30; // Keep 30 seconds of audio context
const CHUNK_SIZE_MS: usize = 500; // Process every 500ms
const CHUNK_SIZE_SAMPLES: usize = SAMPLE_RATE * CHUNK_SIZE_MS / 1000;
const SILENCE_THRESHOLD: f32 = 0.015;

pub struct StreamingTranscriber {
    state: WhisperState,
    audio_window: Vec<f32>,
    last_transcribed_len: usize,
    total_processed: usize,
}

impl StreamingTranscriber {
    pub fn new() -> Self {
        let state = WHISPER
            .create_state()
            .expect("failed to create whisper state");

        Self {
            state,
            audio_window: Vec::with_capacity(SAMPLE_RATE * WINDOW_SIZE_SECONDS),
            last_transcribed_len: 0,
            total_processed: 0,
        }
    }

    fn is_silence(audio: &[f32]) -> bool {
        if audio.is_empty() {
            return true;
        }

        let energy: f32 = audio.iter().map(|s| s.abs()).sum::<f32>() / audio.len() as f32;
        energy < SILENCE_THRESHOLD
    }

    pub fn add_audio(&mut self, samples: &[f32]) {
        // Add new samples to the rolling window
        self.audio_window.extend_from_slice(samples);

        // Keep only the last WINDOW_SIZE_SECONDS of audio
        let max_samples = SAMPLE_RATE * WINDOW_SIZE_SECONDS;
        if self.audio_window.len() > max_samples {
            let excess = self.audio_window.len() - max_samples;
            self.audio_window.drain(0..excess);
            // Adjust the last transcribed position
            self.last_transcribed_len = self.last_transcribed_len.saturating_sub(excess);
        }

        self.total_processed += samples.len();
    }

    pub fn should_transcribe(&self) -> bool {
        // Transcribe when we have enough new audio
        let new_samples = self
            .audio_window
            .len()
            .saturating_sub(self.last_transcribed_len);
        new_samples >= CHUNK_SIZE_SAMPLES
    }

    pub fn transcribe_incremental(&mut self) -> TranscriptionResult {
        if self.audio_window.is_empty() {
            return TranscriptionResult::empty();
        }

        if Self::is_silence(&self.audio_window) {
            return TranscriptionResult::silence();
        }

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_n_threads(4);
        params.set_translate(false);
        params.set_language(Some("auto"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Enable token-level timestamps for better incremental results
        params.set_token_timestamps(true);
        params.set_max_len(1); // Get results as soon as possible

        if self.state.full(params, &self.audio_window).is_err() {
            return TranscriptionResult::error();
        }

        let mut full_text = String::new();
        let mut partial_text = String::new();
        let n_segments = self.state.full_n_segments();

        // Calculate which segments are new
        let new_segment_start = if n_segments > 0 && !self.audio_window.is_empty() {
            ((self.last_transcribed_len * n_segments as usize) / self.audio_window.len()) as i32
        } else {
            0
        };

        for i in 0..n_segments {
            if let Some(segment) = self.state.get_segment(i) {
                if let Ok(seg_text) = segment.to_str() {
                    full_text.push_str(seg_text);

                    // Mark segments after the last transcribed position as partial
                    if i >= new_segment_start {
                        partial_text.push_str(seg_text);
                    }
                }
            }
        }

        // Update the last transcribed length
        self.last_transcribed_len = self.audio_window.len();

        TranscriptionResult {
            full_text: full_text.trim().to_string(),
            partial_text: partial_text.trim().to_string(),
            is_final: false,
            is_silence: false,
        }
    }

    pub fn finalize(&mut self) -> TranscriptionResult {
        if self.audio_window.is_empty() {
            return TranscriptionResult::empty();
        }

        let mut result = self.transcribe_incremental();
        result.is_final = true;
        result
    }

    pub fn reset(&mut self) {
        self.audio_window.clear();
        self.last_transcribed_len = 0;
        self.total_processed = 0;
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub full_text: String,
    pub partial_text: String,
    pub is_final: bool,
    #[allow(dead_code)]
    pub is_silence: bool,
}

impl TranscriptionResult {
    fn empty() -> Self {
        Self {
            full_text: String::new(),
            partial_text: String::new(),
            is_final: false,
            is_silence: false,
        }
    }

    fn silence() -> Self {
        Self {
            full_text: String::new(),
            partial_text: String::new(),
            is_final: false,
            is_silence: true,
        }
    }

    fn error() -> Self {
        Self::empty()
    }

    pub fn has_content(&self) -> bool {
        !self.full_text.is_empty() || !self.partial_text.is_empty()
    }
}
