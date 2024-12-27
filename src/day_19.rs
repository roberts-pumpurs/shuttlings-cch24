use std::{str::FromStr, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    prelude::FromRow,
    types::{
        chrono::{DateTime, Utc},
        Uuid,
    },
    PgPool,
};

/// Converts i64 to a 16-character hex string (uppercase).
fn encode_page(page: i64) -> String {
    format!("{:016X}", page as u64)
}

/// Parses a 16-character hex string back to an i64.
fn decode_page(token: &str) -> Option<i64> {
    let parsed = u64::from_str_radix(token, 16).ok()?;
    Some(parsed as i64)
}

#[derive(Deserialize)]
pub struct Payload {
    author: String,
    quote: String,
}

#[derive(FromRow, Serialize)]
pub struct Quote {
    id: Uuid,
    author: String,
    quote: String,
    created_at: DateTime<Utc>,
    version: i32,
}

#[derive(Serialize)]
pub struct Quotes {
    quotes: Vec<Quote>,
    page: i64,
    next_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    token: String,
}

fn uuid_from_str(s: &str) -> Result<Uuid, StatusCode> {
    Uuid::from_str(s).map_err(|_| StatusCode::BAD_REQUEST)
}

pub async fn reset(State(pool): State<PgPool>) {
    sqlx::query("DELETE FROM quotes")
        .execute(&pool)
        .await
        .unwrap();
}

pub async fn cite(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<Quote>, StatusCode> {
    let id = uuid_from_str(&id)?;
    sqlx::query_as(
        r#"
        SELECT id, author, quote, created_at, version
        FROM quotes
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map(Json)
    .map_err(|_| StatusCode::NOT_FOUND)
}

pub async fn remove(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<Quote>, StatusCode> {
    let id = uuid_from_str(&id)?;
    sqlx::query_as(
        r#"
        DELETE FROM quotes
        WHERE id = $1
        RETURNING id, author, quote, created_at, version
        "#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .map(Json)
    .map_err(|_| StatusCode::NOT_FOUND)
}

pub async fn undo(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(payload): Json<Payload>,
) -> Result<Json<Quote>, StatusCode> {
    let id = uuid_from_str(&id)?;
    sqlx::query_as(
        r#"
        UPDATE quotes
        SET author = $1, quote = $2, version = version+1
        WHERE id = $3
        RETURNING id, author, quote, created_at, version
        "#,
    )
    .bind(payload.author)
    .bind(payload.quote)
    .bind(&id)
    .fetch_one(&pool)
    .await
    .map(Json)
    .map_err(|_| StatusCode::NOT_FOUND)
}

pub async fn draft(
    State(pool): State<PgPool>,
    Json(payload): Json<Payload>,
) -> (StatusCode, Json<Quote>) {
    let quote: Quote = sqlx::query_as(
        r#"
        INSERT INTO quotes (id, author, quote)
        VALUES ($1, $2, $3)
        RETURNING id, author, quote, created_at, version
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(payload.author)
    .bind(payload.quote)
    .fetch_one(&pool)
    .await
    .unwrap();

    (StatusCode::CREATED, Json(quote))
}

pub async fn list(
    State(pool): State<PgPool>,
    query: Option<Query<ListQuery>>,
) -> Result<Json<Quotes>, StatusCode> {
    // If a token is provided, decode the page number; otherwise start at page 0.
    let page_number = if let Some(Query(query)) = query {
        decode_page(&query.token).ok_or(StatusCode::BAD_REQUEST)?
    } else {
        0
    };

    let offset = page_number * 3;

    // Count total quotes in the table
    let (count,): (i64,) = sqlx::query_as(r"SELECT COUNT(id) FROM quotes")
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Only generate a next token if there are more pages
    let next_token = if offset + 3 >= count {
        None
    } else {
        Some(encode_page(page_number + 1))
    };

    let quotes = sqlx::query_as(
        r#"
        SELECT id, author, quote, created_at, version
        FROM quotes
        ORDER BY created_at ASC
        LIMIT 3
        OFFSET $1
        "#,
    )
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(Quotes {
        quotes,
        page: page_number + 1,
        next_token,
    }))
}
