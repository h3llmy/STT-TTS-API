use std::env;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub host: String,
    pub port: String,

    pub model: String,
    pub threads: i32,
    pub use_gpu: bool,

    pub gpu_device: i32,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT").unwrap_or_else(|_| "3078".to_string()),

            model: env::var("STT_MODEL").unwrap_or_else(|_| "ggml-small.en.bin".to_string()),
            threads: env::var("STT_THREADS")
                .unwrap_or_else(|_| "4".to_string())
                .parse()
                .unwrap(),
            use_gpu: env::var("STT_USE_GPU")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap(),

            gpu_device: env::var("GPU_DEVICE")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap(),
        }
    }
}
