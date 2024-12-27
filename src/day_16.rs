use axum::{
    body::{Body, Bytes},
    http::{
        header::{COOKIE, SET_COOKIE},
        HeaderMap, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};

const SECRET: &[u8; 9] = b"my-secret";

pub async fn wrap(Json(claims): Json<serde_json::Value>) -> Response {
    let key = b"secret";
    // Set-Cookie header: gift=(JWT)
    let jwt = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET),
    )
    .unwrap();

    let builder = Response::builder();
    builder
        .header(SET_COOKIE, format!("gift={jwt:}"))
        .body(Body::empty())
        .unwrap()
}

pub async fn unwrap(headers: HeaderMap) -> Response {
    // decode the Cookie: gift=(JWT)
    // if not there, respond with 400
    let Some(cookie_header) = headers.get(COOKIE) else {
        dbg!("content type header not present");
        return (StatusCode::BAD_REQUEST).into_response();
    };

    let token = String::from_utf8_lossy(cookie_header.as_bytes());
    let Some(token) = token.strip_prefix("gift=") else {
        return (StatusCode::BAD_REQUEST).into_response();
    };
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims = Default::default();
    validation.validate_exp = false;

    let token = decode::<serde_json::Value>(&token, &DecodingKey::from_secret(SECRET), &validation);
    dbg!(&token);
    let Ok(token) = token else {
        dbg!("invalid token");
        return (StatusCode::BAD_REQUEST).into_response();
    };

    (StatusCode::OK, token.claims.to_string()).into_response()
}
