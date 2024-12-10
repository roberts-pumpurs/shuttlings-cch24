use std::{
    ops::Div,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Measurement {
    Gallons(f32),
    Liters(f32),
    Litres(f32),
    Pints(f32),
}

static BUCKET_STATE: AtomicU64 = AtomicU64::new(0);
const MAX_BUCKET_SIZE: u8 = 5;
const REFILL_TIME_MS: u64 = 1_000;
const SINGLE_WITHDRAWAL_MILK: u8 = 1;

pub async fn milk(headers: HeaderMap, body: Bytes) -> Response {
    let success_resp = || (StatusCode::OK, "Milk withdrawn\n");
    let no_milk_resp = || (StatusCode::TOO_MANY_REQUESTS, "No milk available\n");
    let bad_req = || (StatusCode::BAD_REQUEST);

    // calculate the amount of time between the last time we withdrew a single milk
    let has_milk = BUCKET_STATE.fetch_update(Ordering::Release, Ordering::Acquire, |old_state| {
        let (old_size, old_ts) = decode_state(old_state);

        // calculate the amount of time between the last time we withdrew a single milk
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let duration_since_last = now - old_ts;

        let delta_to_refill = duration_since_last
            .div(REFILL_TIME_MS)
            .min(MAX_BUCKET_SIZE.into()) as u8;

        if old_size == 0 && delta_to_refill == 0 {
            return None;
        }
        let new_size = (old_size + (delta_to_refill))
            .min(MAX_BUCKET_SIZE)
            .saturating_sub(SINGLE_WITHDRAWAL_MILK);

        Some(encode_state(new_size, now))
    });

    if has_milk.is_err() {
        return no_milk_resp().into_response();
    }

    let is_json = headers
        .get("Content-Type")
        .map(|x| {
            x.to_str()
                .map(|x| x == "application/json")
                .unwrap_or_default()
        })
        .unwrap_or_default();
    if !is_json {
        return success_resp().into_response();
    }
    let Ok(measurements) = serde_json::from_slice::<Measurement>(&body) else {
        return bad_req().into_response();
    };
    let new_measurement = match measurements {
        Measurement::Gallons(val) => Measurement::Liters(val * 3.78541),
        Measurement::Liters(val) => Measurement::Gallons(val * (1.0 / 3.78541)),
        Measurement::Litres(val) => Measurement::Pints(val * 1.75975),
        Measurement::Pints(val) => Measurement::Litres(val * (1.0 / 1.75975)),
    };
    (
        StatusCode::OK,
        serde_json::to_string(&new_measurement).unwrap(),
    )
        .into_response()
}

pub async fn refill() -> Response {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let new_state = encode_state(MAX_BUCKET_SIZE as u8, now);
    BUCKET_STATE.swap(new_state, Ordering::AcqRel);
    (StatusCode::OK,).into_response()
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
