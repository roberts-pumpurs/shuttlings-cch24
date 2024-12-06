use std::{
    net::{Ipv4Addr, Ipv6Addr},
    ops::BitXor,
};

use axum::{
    body::{Body, Bytes},
    extract::Query,
    handler::HandlerWithoutStateExt,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use itertools::Itertools;

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

#[derive(serde::Deserialize)]
struct V6DestQParams {
    from: Ipv6Addr,
    key: Ipv6Addr,
}

async fn v6_dest(params: Query<V6DestQParams>) -> String {
    let mut segments = [0; 16];
    for (idx, (from, key)) in params
        .from
        .octets()
        .into_iter()
        .zip_eq(params.key.octets())
        .enumerate()
    {
        let value = from.bitxor(key);
        segments[idx] = value;
    }
    let destination = Ipv6Addr::from(segments);

    destination.to_string()
}

#[derive(serde::Deserialize)]
struct V6KeyQParams {
    from: Ipv6Addr,
    to: Ipv6Addr,
}
async fn v6_key(params: Query<V6KeyQParams>) -> String {
    let mut segments = [0; 16];
    for (idx, (to, from)) in params
        .to
        .octets()
        .into_iter()
        .zip_eq(params.from.octets())
        .enumerate()
    {
        let value = to.bitxor(from);
        segments[idx] = value;
    }
    let destination = Ipv6Addr::from(segments);

    destination.to_string()
}

#[derive(serde::Deserialize, Debug)]
struct Metadata {
    orders: Vec<Order>,
}

#[derive(serde::Deserialize, Debug)]
struct Order {
    item: Option<toml::Value>,
    quantity: Option<toml::Value>,
}

#[axum::debug_handler]
async fn manifest(headers: HeaderMap, body: Bytes) -> Response {
    let invalid_response = || Response::builder().status(204).body(Body::empty()).unwrap();
    let invalid_manifest = || Response::builder().status(400).body(Body::empty()).unwrap();

    let Some(content_type) = headers.get("Content-Type") else {
        dbg!("content type header not present");
        return invalid_response();
    };
    if content_type != "application/toml" {
        dbg!("invalid content type");
        return invalid_response();
    }

    let manifest = cargo_manifest::Manifest::<Metadata>::from_slice_with_metadata(&body);
    let Ok(manifest) = manifest else {
        dbg!(&manifest);
        dbg!("invalid manifest");
        return invalid_manifest();
    };

    let Some(metadata) = manifest.package.and_then(|m| m.metadata) else {
        dbg!("metadata manifest key not present");
        return invalid_response();
    };

    let (counter, valid_orders) = metadata
        .orders
        .into_iter()
        .filter_map(|order| {
            let item = match order.item? {
                toml::Value::String(s) => s,
                _ => None?,
            };
            let quantity: u32 = match order.quantity? {
                toml::Value::Integer(integer) => integer.try_into().ok()?,
                _ => None?,
            };
            Some((item, quantity))
        })
        .fold((0, "".to_owned()), |(mut counter, mut acc), i| {
            if counter > 0 {
                acc.push_str("\n");
            }
            acc.push_str(i.0.as_str());
            acc.push_str(": ");
            acc.push_str(i.1.to_string().as_str());
            counter += 1;
            (counter, acc)
        });

    if counter == 0 {
        dbg!("no valid orders");
        return invalid_response();
    };

    dbg!(&valid_orders);
    Response::builder()
        .status(200)
        .body(Body::new(valid_orders))
        .unwrap()
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(hello_world))
        .route("/-1/seek", get(seek))
        .route("/2/dest", get(dest))
        .route("/2/key", get(key))
        .route("/2/v6/dest", get(v6_dest))
        .route("/2/v6/key", get(v6_key))
        .route("/5/manifest", post(manifest));

    Ok(router.into())
}
