use once_cell::sync::Lazy;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub static WHISPER: Lazy<WhisperContext> = Lazy::new(|| {
    println!("Loading Whisper model...");
    let ctx = WhisperContext::new_with_params(
        "models/ggml-large-v3-turbo.bin",
        WhisperContextParameters::default(),
    )
    .expect("failed to load model");

    println!("Whisper model loaded.");
    ctx
});

fn is_silence(audio: &[f32]) -> bool {
    if audio.is_empty() {
        return true;
    }

    let energy: f32 = audio.iter().map(|s| s.abs()).sum::<f32>() / audio.len() as f32;

    energy < 0.015
}

pub fn transcribe(audio: &[f32]) -> Option<String> {
    if is_silence(audio) {
        return None;
    }

    let mut state = WHISPER
        .create_state()
        .expect("failed to create whisper state");

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    params.set_n_threads(4);
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
