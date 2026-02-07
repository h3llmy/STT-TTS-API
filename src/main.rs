mod domain;

use axum::Router;
use once_cell::sync::Lazy;

use crate::domain::stt::service::WHISPER;

#[tokio::main]
async fn main() {
    // 🔥 Force model to load at startup
    Lazy::force(&WHISPER);

    let app = Router::new().merge(domain::stt::route::route());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3078").await.unwrap();

    println!("Server running on http://localhost:3078");

    axum::serve(listener, app).await.unwrap();
}
