use axum::{
    routing::{get, post},
    Router,
};

mod day_1;
mod day_2;
mod day_5;
mod day_9;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(day_1::hello_world))
        .route("/-1/seek", get(day_1::seek))
        .route("/2/dest", get(day_2::dest))
        .route("/2/key", get(day_2::key))
        .route("/2/v6/dest", get(day_2::v6_dest))
        .route("/2/v6/key", get(day_2::v6_key))
        .route("/5/manifest", post(day_5::manifest))
        .route("/9/milk", post(day_9::milk))
        .route("/9/refill", post(day_9::refill));

    Ok(router.into())
}
