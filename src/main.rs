use axum::{
    body::Body,
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

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(hello_world))
        .route("/-1/seek", get(seek));

    Ok(router.into())
}
