use std::net::Ipv4Addr;

use axum::{
    body::Body,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

async fn hello_world() -> &'static str {
    "Hello, bird!"
}

async fn seek() -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", "https://www.youtube.com/watch?v=9Gc4QTqslN4")
        .body(Body::empty())
        .unwrap()
}

#[derive(serde::Deserialize)]
struct DestQParams {
    from: Ipv4Addr,
    key: Ipv4Addr,
}

async fn dest(params: Query<DestQParams>) -> String {
    let octets = [
        params.from.octets()[0]
            .overflowing_add(params.key.octets()[0])
            .0,
        params.from.octets()[1]
            .overflowing_add(params.key.octets()[1])
            .0,
        params.from.octets()[2]
            .overflowing_add(params.key.octets()[2])
            .0,
        params.from.octets()[3]
            .overflowing_add(params.key.octets()[3])
            .0,
    ];
    let destination = Ipv4Addr::from(octets);

    destination.to_string()
}

#[derive(serde::Deserialize)]
struct KeyQParams {
    from: Ipv4Addr,
    to: Ipv4Addr,
}
async fn key(params: Query<KeyQParams>) -> String {
    let octets = [
        params.to.octets()[0]
            .overflowing_sub(params.from.octets()[0])
            .0,
        params.to.octets()[1]
            .overflowing_sub(params.from.octets()[1])
            .0,
        params.to.octets()[2]
            .overflowing_sub(params.from.octets()[2])
            .0,
        params.to.octets()[3]
            .overflowing_sub(params.from.octets()[3])
            .0,
    ];
    let destination = Ipv4Addr::from(octets);

    destination.to_string()
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(hello_world))
        .route("/-1/seek", get(seek))
        .route("/2/dest", get(dest))
        .route("/2/key", get(key));

    Ok(router.into())
}
