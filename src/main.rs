mod domain;

use std::env;

use axum::Router;
use once_cell::sync::Lazy;

use crate::domain::stt::service::WHISPER;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    // 🔥 Force models to load at startup
    Lazy::force(&WHISPER);
    crate::domain::tts::service::init_tts().await;

    let app = Router::new()
        .merge(domain::stt::route::route())
        .merge(domain::tts::route::route());

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3078".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("Server running on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
