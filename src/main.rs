use std::net::SocketAddr;
use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod models;
mod state;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("jourloc=info".parse()?))
        .init();

    let config = Config::from_env()?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("failed to connect to Postgres")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    let state = AppState::new(pool, config.clone());
    let public_service = ServeDir::new("public");

    let app = Router::new()
        .route("/", get(handlers::index))
        .route("/api/health", get(handlers::health))
        .route("/api/me", get(handlers::me))
        .route("/api/login", post(handlers::login))
        .route("/api/logout", post(handlers::logout))
        .route("/api/pages", get(handlers::list_pages).post(handlers::create_page))
        .route(
            "/api/pages/:id",
            get(handlers::get_page)
                .put(handlers::update_page)
                .delete(handlers::delete_page),
        )
        .route("/api/tags", get(handlers::list_tags))
        .fallback_service(public_service)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind server port")?;

    tracing::info!("JourLoc listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .context("server error")?;

    Ok(())
}
