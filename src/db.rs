use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::models::{PageDetail, PagePayload, PageQuery, PageSummary};

pub async fn list_pages_query(pool: &PgPool, query: PageQuery) -> Result<Vec<PageSummary>> {
    let search = query.search.unwrap_or_default();
    let tag = query.tag.unwrap_or_default();

    let rows = sqlx::query_as::<_, PageSummary>(
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

    Ok(rows)
}

pub async fn fetch_page(pool: &PgPool, id: i32) -> Result<Option<PageDetail>> {
    let row = sqlx::query_as::<_, PageDetail>(
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

    Ok(row)
}

pub async fn create_page_query(pool: &PgPool, payload: PagePayload) -> Result<PageDetail> {
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

pub async fn update_page_query(
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

pub async fn delete_page_query(pool: &PgPool, id: i32) -> Result<bool> {
    let result = sqlx::query("DELETE FROM pages WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_tags_query(pool: &PgPool) -> Result<Vec<String>> {
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
