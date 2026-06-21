use sqlx::PgPool;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub app_password: String,
    pub session_secret: String,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self {
            pool,
            app_password: config.app_password,
            session_secret: config.session_secret,
        }
    }
}
