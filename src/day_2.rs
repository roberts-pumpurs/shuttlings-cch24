use std::{
    net::{Ipv4Addr, Ipv6Addr},
    ops::BitXor,
};

use axum::extract::Query;
use itertools::Itertools;

#[derive(serde::Deserialize)]
pub struct DestQParams {
    from: Ipv4Addr,
    key: Ipv4Addr,
}

pub async fn dest(params: Query<DestQParams>) -> String {
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
pub struct KeyQParams {
    from: Ipv4Addr,
    to: Ipv4Addr,
}
pub async fn key(params: Query<KeyQParams>) -> String {
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
pub struct V6DestQParams {
    from: Ipv6Addr,
    key: Ipv6Addr,
}

pub async fn v6_dest(params: Query<V6DestQParams>) -> String {
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
pub struct V6KeyQParams {
    from: Ipv6Addr,
    to: Ipv6Addr,
}
pub async fn v6_key(params: Query<V6KeyQParams>) -> String {
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
