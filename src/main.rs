use std::{env, net::SocketAddr};

use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cookie::time::Duration as CookieDuration;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    app_password: String,
    session_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    authenticated: bool,
}

#[derive(Debug, Serialize)]
struct MeResponse {
    authenticated: bool,
}

#[derive(Debug, Deserialize)]
struct PageQuery {
    search: Option<String>,
    tag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PagePayload {
    title: String,
    content: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct PageSummary {
    id: i32,
    title: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PageDetail {
    id: i32,
    title: String,
    content: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    tags: Vec<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct PageSummaryRow {
    id: i32,
    title: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    tags: Vec<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct PageDetailRow {
    id: i32,
    title: String,
    content: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    tags: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("jourloc=info".parse()?))
        .init();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let app_password = env::var("APP_PASSWORD").context("APP_PASSWORD must be set")?;
    let session_secret = env::var("SESSION_SECRET").context("SESSION_SECRET must be set")?;
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .context("failed to connect to Postgres")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")?;

    let state = AppState {
        pool,
        app_password,
        session_secret,
    };

    let public_service = ServeDir::new("public");

    let app = Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/me", get(me))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/pages", get(list_pages).post(create_page))
        .route("/api/pages/:id", get(get_page).put(update_page).delete(delete_page))
        .route("/api/tags", get(list_tags))
        .fallback_service(public_service)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("failed to bind server port")?;

    tracing::info!("JourLoc listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .context("server error")?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn index() -> impl IntoResponse {
    Html(include_str!("../public/index.html"))
}

async fn me(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    Json(MeResponse {
        authenticated: is_authenticated(&jar, &state.session_secret),
    })
}

async fn login(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Response {
    if payload.password != state.app_password {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid password"})),
        )
            .into_response();
    }

    let token = create_session_token(&state.session_secret);
    let cookie = Cookie::build(("jourloc_session", token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::days(7))
        .build();

    let jar = jar.add(cookie);
    (jar, Json(LoginResponse { authenticated: true })).into_response()
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(("jourloc_session", ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::seconds(0))
        .build();

    let jar = jar.remove(cookie);
    (jar, Json(serde_json::json!({"ok": true})))
}

async fn list_pages(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<PageQuery>,
) -> Response {
    if !require_auth(&jar, &state.session_secret) {
        return unauthorized();
    }

    match list_pages_query(&state.pool, query).await {
        Ok(pages) => Json(serde_json::json!({"pages": pages})).into_response(),
        Err(error) => internal_error(error),
    }
}

async fn get_page(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Response {
    if !require_auth(&jar, &state.session_secret) {
        return unauthorized();
    }

    match fetch_page(&state.pool, id).await {
        Ok(Some(page)) => Json(serde_json::json!({"page": page})).into_response(),
        Ok(None) => not_found("Page not found"),
        Err(error) => internal_error(error),
    }
}

async fn create_page(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<PagePayload>,
) -> Response {
    if !require_auth(&jar, &state.session_secret) {
        return unauthorized();
    }

    if payload.title.trim().is_empty() {
        return bad_request("Title is required");
    }

    match create_page_query(&state.pool, payload).await {
        Ok(page) => Json(serde_json::json!({"page": page})).into_response(),
        Err(error) => internal_error(error),
    }
}

async fn update_page(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<PagePayload>,
) -> Response {
    if !require_auth(&jar, &state.session_secret) {
        return unauthorized();
    }

    if payload.title.trim().is_empty() {
        return bad_request("Title is required");
    }

    match update_page_query(&state.pool, id, payload).await {
        Ok(Some(page)) => Json(serde_json::json!({"page": page})).into_response(),
        Ok(None) => not_found("Page not found"),
        Err(error) => internal_error(error),
    }
}

async fn delete_page(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Response {
    if !require_auth(&jar, &state.session_secret) {
        return unauthorized();
    }

    match delete_page_query(&state.pool, id).await {
        Ok(true) => Json(serde_json::json!({"ok": true})).into_response(),
        Ok(false) => not_found("Page not found"),
        Err(error) => internal_error(error),
    }
}

async fn list_tags(jar: CookieJar, State(state): State<AppState>) -> Response {
    if !require_auth(&jar, &state.session_secret) {
        return unauthorized();
    }

    match list_tags_query(&state.pool).await {
        Ok(tags) => Json(serde_json::json!({"tags": tags})).into_response(),
        Err(error) => internal_error(error),
    }
}

async fn list_pages_query(pool: &PgPool, query: PageQuery) -> Result<Vec<PageSummary>> {
    let search = query.search.unwrap_or_default();
    let tag = query.tag.unwrap_or_default();

    let rows = sqlx::query_as::<_, PageSummaryRow>(
        r#"
        SELECT p.id, p.title, p.created_at, p.updated_at,
               COALESCE(t.tags, ARRAY[]::text[]) AS tags
        FROM pages p
        LEFT JOIN (
            SELECT pt.page_id, array_agg(t.name ORDER BY t.name) AS tags
            FROM page_tags pt
            JOIN tags t ON t.id = pt.tag_id
            GROUP BY pt.page_id
        ) t ON t.page_id = p.id
        WHERE ($1 = '' OR p.title ILIKE '%' || $1 || '%')
          AND ($2 = '' OR EXISTS (
              SELECT 1
              FROM page_tags pt2
              JOIN tags t2 ON t2.id = pt2.tag_id
              WHERE pt2.page_id = p.id AND t2.name = $2
          ))
        ORDER BY p.updated_at DESC
        "#,
    )
    .bind(search)
    .bind(tag)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PageSummary {
            id: row.id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
            tags: row.tags,
        })
        .collect())
}

async fn fetch_page(pool: &PgPool, id: i32) -> Result<Option<PageDetail>> {
    let row = sqlx::query_as::<_, PageDetailRow>(
        r#"
        SELECT p.id, p.title, p.content, p.created_at, p.updated_at,
               COALESCE(t.tags, ARRAY[]::text[]) AS tags
        FROM pages p
        LEFT JOIN (
            SELECT pt.page_id, array_agg(t.name ORDER BY t.name) AS tags
            FROM page_tags pt
            JOIN tags t ON t.id = pt.tag_id
            GROUP BY pt.page_id
        ) t ON t.page_id = p.id
        WHERE p.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| PageDetail {
        id: row.id,
        title: row.title,
        content: row.content,
        created_at: row.created_at,
        updated_at: row.updated_at,
        tags: row.tags,
    }))
}

async fn create_page_query(pool: &PgPool, payload: PagePayload) -> Result<PageDetail> {
    let mut tx = pool.begin().await?;
    let tags = normalize_tags(payload.tags.unwrap_or_default());
    let content = payload.content.unwrap_or_default();

    let id = sqlx::query(
        r#"
        INSERT INTO pages (title, content)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(payload.title.trim())
    .bind(content)
    .fetch_one(&mut *tx)
    .await?
    .try_get::<i32, _>("id")?;

    replace_tags(&mut tx, id, &tags).await?;
    tx.commit().await?;

    fetch_page(pool, id)
        .await?
        .context("page should exist right after creation")
}

async fn update_page_query(
    pool: &PgPool,
    id: i32,
    payload: PagePayload,
) -> Result<Option<PageDetail>> {
    let mut tx = pool.begin().await?;
    let tags = normalize_tags(payload.tags.unwrap_or_default());
    let content = payload.content.unwrap_or_default();

    let updated = sqlx::query(
        r#"
        UPDATE pages
        SET title = $1, content = $2, updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(payload.title.trim())
    .bind(content)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if updated.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(None);
    }

    replace_tags(&mut tx, id, &tags).await?;
    tx.commit().await?;

    Ok(fetch_page(pool, id).await?)
}

async fn delete_page_query(pool: &PgPool, id: i32) -> Result<bool> {
    let result = sqlx::query("DELETE FROM pages WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

async fn list_tags_query(pool: &PgPool) -> Result<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT t.name
        FROM tags t
        JOIN page_tags pt ON pt.tag_id = t.id
        GROUP BY t.name
        ORDER BY t.name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| row.try_get::<String, _>("name"))
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

async fn replace_tags(
    tx: &mut Transaction<'_, Postgres>,
    page_id: i32,
    tags: &[String],
) -> Result<()> {
    sqlx::query("DELETE FROM page_tags WHERE page_id = $1")
        .bind(page_id)
        .execute(&mut **tx)
        .await?;

    for tag in tags {
        let tag_id = sqlx::query(
            r#"
            INSERT INTO tags (name)
            VALUES ($1)
            ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
        )
        .bind(tag)
        .fetch_one(&mut **tx)
        .await?
        .try_get::<i32, _>("id")?;

        sqlx::query(
            r#"
            INSERT INTO page_tags (page_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(page_id)
        .bind(tag_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let trimmed = tag.trim();
        if !trimmed.is_empty() && !normalized.iter().any(|existing: &String| existing == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

fn create_session_token(secret: &str) -> String {
    let exp = (Utc::now() + ChronoDuration::days(7)).timestamp() as usize;
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

fn is_authenticated(jar: &CookieJar, secret: &str) -> bool {
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

fn require_auth(jar: &CookieJar, secret: &str) -> bool {
    is_authenticated(jar, secret)
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Not authenticated"}))).into_response()
}

fn bad_request(message: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": message}))).into_response()
}

fn not_found(message: &str) -> Response {
    (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": message}))).into_response()
}

fn internal_error(error: impl std::fmt::Debug) -> Response {
    tracing::error!(error = ?error, "request failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "Internal server error"})),
    )
        .into_response()
}
