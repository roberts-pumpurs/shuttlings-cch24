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
    decode as jwt_decode, decode_header, encode, errors::ErrorKind, Algorithm, DecodingKey,
    EncodingKey, Header, Validation,
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

    let token =
        jwt_decode::<serde_json::Value>(&token, &DecodingKey::from_secret(SECRET), &validation);
    dbg!(&token);
    let Ok(token) = token else {
        dbg!("invalid token");
        return (StatusCode::BAD_REQUEST).into_response();
    };

    (StatusCode::OK, token.claims.to_string()).into_response()
}

pub async fn decode(body: Bytes) -> Result<Json<serde_json::Value>, StatusCode> {
    let jwt = String::from_utf8_lossy(&body);
    dbg!(&jwt);
    let key = include_bytes!("../day16_santa_public_key.pem");
    let header = decode_header(&jwt).map_err(|_| StatusCode::BAD_REQUEST)?;
    dbg!(&header);
    let mut validation = Validation::default();
    validation.algorithms = vec![header.alg];
    validation.required_spec_claims.remove("exp");

    let key = DecodingKey::from_rsa_pem(key).unwrap();
    let token = jwt_decode(&jwt, &key, &validation).map_err(|e| match e.into_kind() {
        ErrorKind::InvalidSignature => StatusCode::UNAUTHORIZED,
        _ => StatusCode::BAD_REQUEST,
    })?;
    Ok(Json(token.claims))
}
