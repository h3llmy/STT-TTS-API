use tokio::net::TcpListener;

use crate::core::config::Config;

mod app;
mod core;
mod domain;
mod infrastructure;
mod presentation;
mod shared;

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    let app = app::build_app(&config).await;

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("Server running on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
