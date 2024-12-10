use axum::{body::Body, http::StatusCode, response::Response};

pub async fn hello_world() -> &'static str {
    "Hello, bird!"
}

pub async fn seek() -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", "https://www.youtube.com/watch?v=9Gc4QTqslN4")
        .body(Body::empty())
        .unwrap()
}
