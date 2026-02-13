use hound::{SampleFormat, WavSpec, WavWriter};
use kokoro_tiny::TtsEngine;
use std::io::Cursor;
use std::sync::Mutex;
use tokio::sync::OnceCell;

pub static TTS_ENGINE: OnceCell<Mutex<TtsEngine>> = OnceCell::const_new();

const SAMPLE_RATE: u32 = 24_000;
const SILENCE_PADDING_MS: f32 = 0.2; // 200ms safety padding

// ================= INIT =================

pub async fn init_tts() {
    println!("Loading Kokoro TTS engine...");

    let mut engine = TtsEngine::new()
        .await
        .expect("Failed to initialize Kokoro TTS engine");

    // 🔥 Warmup to prevent first-token drop
    println!("Warming up TTS engine...");
    let _ = engine.synthesize("Hello world", Some("af_sky"));

    println!("Kokoro TTS engine loaded.");

    let _ = TTS_ENGINE.set(Mutex::new(engine));
}

// ================= SYNTHESIZE =================

pub fn synthesize(text: &str, voice: Option<&str>) -> Vec<u8> {
    let engine_mutex = TTS_ENGINE.get().expect("TTS engine not initialized");

    let mut engine = engine_mutex.lock().expect("Failed to lock TTS engine");

    let voice_name = voice.unwrap_or("af_sky");

    // 🔥 Basic text normalization
    let clean_text = normalize_text(text);

    println!("Synthesizing: {}", clean_text);

    match engine.synthesize(&clean_text, Some(voice_name)) {
        Ok(mut samples) => {
            // 🔥 Add silence padding to prevent first word cut
            let silence_samples = (SAMPLE_RATE as f32 * SILENCE_PADDING_MS) as usize;
            let mut padded_samples = vec![0.0f32; silence_samples];
            padded_samples.append(&mut samples);

            write_wav(padded_samples)
        }
        Err(e) => {
            eprintln!("TTS synthesis error: {:?}", e);
            Vec::new()
        }
    }
}

// ================= WAV WRITER =================

fn write_wav(samples: Vec<f32>) -> Vec<u8> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());

    {
        let mut writer = WavWriter::new(&mut cursor, spec).expect("Failed to create WAV writer");

        for sample in samples {
            // Convert f32 (-1.0..1.0) → i16
            let pcm_sample = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;

            writer
                .write_sample(pcm_sample)
                .expect("Failed to write sample");
        }

        writer.finalize().expect("Failed to finalize WAV");
    }

    cursor.into_inner()
}

// ================= TEXT CLEANING =================

fn normalize_text(input: &str) -> String {
    input.trim().replace("halo", "hello") // optional EN fix
}
