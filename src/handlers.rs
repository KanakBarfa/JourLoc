use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use cookie::time::Duration as CookieDuration;

use crate::{
    auth::{create_session_token, is_authenticated},
    db,
    error::AppError,
    models::{Claims, LoginRequest, LoginResponse, MeResponse, PagePayload, PageQuery},
    state::AppState,
};

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn index() -> impl IntoResponse {
    Html(include_str!("../public/index.html"))
}

pub async fn me(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    Json(MeResponse {
        authenticated: is_authenticated(&jar, &state.session_secret),
    })
}

pub async fn login(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, AppError> {
    if payload.password != state.app_password {
        return Ok((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid password" })),
        )
            .into_response());
    }

    let token = create_session_token(&state.session_secret);
    let cookie = Cookie::build(("jourloc_session", token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::days(7))
        .build();

    let jar = jar.add(cookie);
    Ok((jar, Json(LoginResponse { authenticated: true })).into_response())
}

pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(("jourloc_session", ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::seconds(0))
        .build();

    let jar = jar.remove(cookie);
    (jar, Json(serde_json::json!({ "ok": true })))
}

pub async fn list_pages(
    _claims: Claims,
    State(state): State<AppState>,
    Query(query): Query<PageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let pages = db::list_pages_query(&state.pool, query).await?;
    Ok(Json(serde_json::json!({ "pages": pages })))
}

pub async fn get_page(
    _claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    match db::fetch_page(&state.pool, id).await? {
        Some(page) => Ok(Json(serde_json::json!({ "page": page })).into_response()),
        None => Err(AppError::NotFound("Page not found".to_string())),
    }
}

pub async fn create_page(
    _claims: Claims,
    State(state): State<AppState>,
    Json(payload): Json<PagePayload>,
) -> Result<impl IntoResponse, AppError> {
    if payload.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title is required".to_string()));
    }

    let page = db::create_page_query(&state.pool, payload).await?;
    Ok(Json(serde_json::json!({ "page": page })))
}

pub async fn update_page(
    _claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<PagePayload>,
) -> Result<impl IntoResponse, AppError> {
    if payload.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title is required".to_string()));
    }

    match db::update_page_query(&state.pool, id, payload).await? {
        Some(page) => Ok(Json(serde_json::json!({ "page": page })).into_response()),
        None => Err(AppError::NotFound("Page not found".to_string())),
    }
}

pub async fn delete_page(
    _claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    match db::delete_page_query(&state.pool, id).await? {
        true => Ok(Json(serde_json::json!({ "ok": true })).into_response()),
        false => Err(AppError::NotFound("Page not found".to_string())),
    }
}

pub async fn list_tags(
    _claims: Claims,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let tags = db::list_tags_query(&state.pool).await?;
    Ok(Json(serde_json::json!({ "tags": tags })))
}
