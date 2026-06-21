use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::{error::AppError, models::Claims, state::AppState};

pub fn create_session_token(secret: &str) -> String {
    let exp = (Utc::now() + chrono::Duration::days(7)).timestamp() as usize;
    let claims = Claims {
        sub: "jourloc".to_string(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("failed to encode jwt")
}

pub fn is_authenticated(jar: &CookieJar, secret: &str) -> bool {
    let Some(cookie) = jar.get("jourloc_session") else {
        return false;
    };

    decode::<Claims>(
        cookie.value(),
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .is_ok()
}

#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Unauthorized)?;

        let cookie = jar.get("jourloc_session").ok_or(AppError::Unauthorized)?;

        let token_data = decode::<Claims>(
            cookie.value(),
            &DecodingKey::from_secret(state.session_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        Ok(token_data.claims)
    }
}
