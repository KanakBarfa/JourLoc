use std::env;
use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub app_password: String,
    pub session_secret: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let app_password = env::var("APP_PASSWORD").context("APP_PASSWORD must be set")?;
        let session_secret = env::var("SESSION_SECRET").context("SESSION_SECRET must be set")?;
        let port: u16 = env::var("PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3000);

        Ok(Config {
            database_url,
            app_password,
            session_secret,
            port,
        })
    }
}
