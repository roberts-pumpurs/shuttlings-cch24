use std::{
    net::{Ipv4Addr, Ipv6Addr},
    ops::{BitXor, Div},
    sync::atomic::{AtomicPtr, AtomicU64, AtomicU8, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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
    orders: Option<Vec<Order>>,
}

#[derive(serde::Deserialize, Debug)]
struct Order {
    item: Option<toml::Value>,
    quantity: Option<toml::Value>,
}

#[axum::debug_handler]
async fn manifest(headers: HeaderMap, body: Bytes) -> Response {
    let invalid_response = || Response::builder().status(204).body(Body::empty()).unwrap();
    let invalid_media_type = || Response::builder().status(415).body(Body::empty()).unwrap();
    let invalid_manifest = || {
        Response::builder()
            .status(400)
            .body(Body::new("Invalid manifest".to_string()))
            .unwrap()
    };
    let magic_keywrod_not_present = || {
        Response::builder()
            .status(400)
            .body(Body::new("Magic keyword not provided".to_string()))
            .unwrap()
    };

    let Some(content_type) = headers.get("Content-Type") else {
        dbg!("content type header not present");
        return invalid_response();
    };

    let manifest = match content_type.to_str().unwrap_or("") {
        "application/toml" => {
            let Ok(metadata) =
                cargo_manifest::Manifest::<Metadata>::from_slice_with_metadata(&body)
            else {
                return invalid_manifest();
            };
            metadata
        }
        "application/yaml" => {
            let Ok(metadata) = serde_yaml::from_slice::<cargo_manifest::Manifest<Metadata>>(&body)
            else {
                return invalid_manifest();
            };
            metadata
        }
        "application/json" => {
            let Ok(metadata) = serde_json::from_slice::<cargo_manifest::Manifest<Metadata>>(&body)
            else {
                return invalid_manifest();
            };
            metadata
        }
        _ => return invalid_media_type(),
    };

    let has_magic_keyword = manifest
        .package
        .as_ref()
        .and_then(|x| x.keywords.as_ref())
        .map(|x| match x {
            cargo_manifest::MaybeInherited::Inherited { .. } => false,
            cargo_manifest::MaybeInherited::Local(keyw) => keyw
                .into_iter()
                .find(|x| x.as_str() == "Christmas 2024")
                .is_some(),
        })
        .unwrap_or_default();
    if !has_magic_keyword {
        return magic_keywrod_not_present();
    }

    let Some(metadata) = manifest.package.and_then(|m| m.metadata) else {
        dbg!("metadata manifest key not present");
        return invalid_response();
    };

    let Some(orders) = metadata.orders else {
        dbg!("metadata orders key not present");
        return invalid_response();
    };

    let (counter, valid_orders) = orders
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

fn encode_state(bucket_size: u8, timestamp_ms: u64) -> u64 {
    let mut encoded = [0_u8; 8];
    for (idx, byte) in timestamp_ms.to_le_bytes().into_iter().enumerate().skip(1) {
        encoded[idx] = byte;
    }
    encoded[0] = bucket_size;
    u64::from_le_bytes(encoded)
}

fn decode_state(state: u64) -> (u8, u64) {
    let mut bytes = state.to_le_bytes();
    let bucket_size = bytes[0];
    let timestamp_ms = {
        bytes[0] = 0;
        u64::from_le_bytes(bytes)
    };
    (bucket_size, timestamp_ms)
}

#[test]
fn test_encode_round_trip() {
    let bucket_size = 10;
    let timestamp_ms = 1_614_000_000_000u64; // Example timestamp
    let encoded = encode_state(bucket_size, timestamp_ms);
    let (d_bucket_size, d_timestamp_ms) = decode_state(encoded);

    assert_eq!(bucket_size, d_bucket_size,);
    assert_eq!(timestamp_ms, d_timestamp_ms,);
}

async fn milk() -> Response {
    const MAX_BUCKET_SIZE: u64 = 5;
    const REFILL_TIME_MS: u64 = 1_000;
    const SINGLE_WITHDRAWAL_MILK: u64 = 1;

    static BUCKET_STATE: AtomicU64 = AtomicU64::new(0);

    let success_resp = || {
        Response::builder()
            .status(200)
            .body(Body::new("Milk withdrawn\n".to_string()))
            .unwrap()
    };
    let no_milk_resp = || {
        Response::builder()
            .status(429)
            .body(Body::new("No milk available\n".to_string()))
            .unwrap()
    };

    // calculate the amount of time between the last time we withdrew a single milk
    let has_milk = BUCKET_STATE.fetch_update(Ordering::Release, Ordering::Acquire, |old_state| {
        let (old_size, old_ts) = decode_state(old_state);
        dbg!(old_size);
        dbg!(old_ts);

        // calculate the amount of time between the last time we withdrew a single milk
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let duration_since_last = now - old_ts;

        let delta_to_refill = duration_since_last.div(REFILL_TIME_MS).min(MAX_BUCKET_SIZE);
        dbg!(delta_to_refill);

        if old_size == 0 && delta_to_refill == 0 {
            return None;
        }
        let new_size = (old_size + (delta_to_refill as u8))
            .min(MAX_BUCKET_SIZE as u8)
            .saturating_sub(SINGLE_WITHDRAWAL_MILK as u8);
        dbg!(new_size);

        Some(encode_state(new_size, now))
    });

    if has_milk.is_ok() {
        return success_resp();
    }

    return no_milk_resp();
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
        .route("/5/manifest", post(manifest))
        .route("/9/milk", post(milk));

    Ok(router.into())
}
