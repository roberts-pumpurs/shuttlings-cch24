#![feature(portable_simd)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    body::Body,
    http::{Request, Response},
    routing::{delete, get, post, put},
    Router,
};
use rand::SeedableRng;
use tower_http::trace::TraceLayer;
use tracing::Span;

mod day_1;
mod day_12;
mod day_16;
mod day_19;
mod day_2;
mod day_5;
mod day_9;

#[shuttle_runtime::main]
async fn main(#[shuttle_shared_db::Postgres] pool: sqlx::PgPool) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let std_rng = rand::rngs::StdRng::seed_from_u64(2024);

    let router = Router::new()
        .route("/", get(day_1::hello_world))
        .route("/-1/seek", get(day_1::seek))
        .route("/2/dest", get(day_2::dest))
        .route("/2/key", get(day_2::key))
        .route("/2/v6/dest", get(day_2::v6_dest))
        .route("/2/v6/key", get(day_2::v6_key))
        .route("/5/manifest", post(day_5::manifest))
        .route("/9/milk", post(day_9::milk))
        .route("/9/refill", post(day_9::refill))
        .route("/12/board", get(day_12::board))
        .route("/12/reset", post(day_12::reset))
        .route("/12/place/:team/:column", post(day_12::place))
        .route("/12/random-board", get(day_12::random_board))
        .with_state(Arc::new(Mutex::new(std_rng)))
        .route("/16/wrap", post(day_16::wrap))
        .route("/16/unwrap", get(day_16::unwrap))
        .route("/16/decode", post(day_16::decode))
        .route("/19/reset", post(day_19::reset))
        .route("/19/cite/:id", get(day_19::cite))
        .route("/19/remove/:id", delete(day_19::remove))
        .route("/19/undo/:id", put(day_19::undo))
        .route("/19/draft", post(day_19::draft))
        .route("/19/list", get(day_19::list))
        .with_state(pool)
        .layer(TraceLayer::new_for_http().make_span_with(|req: &Request<Body>| {
            tracing::info_span!("", method = %req.method(), uri = %req.uri())
        }).on_response(|res: &Response<Body>, latency: Duration, _span: &Span| {
            if res.status().is_server_error() {
                tracing::error!(status = %res.status().as_u16(), latency = ?latency);
            } else if res.status().is_client_error() {
                tracing::warn!(status = %res.status().as_u16(), latency = ?latency);
            } else {
                tracing::info!(status = %res.status().as_u16(), latency = ?latency);
            }
        }).on_failure(()));

    Ok(router.into())
}
