use axum::{
    body::{Body, Bytes},
    http::HeaderMap,
    response::Response,
};

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
pub async fn manifest(headers: HeaderMap, body: Bytes) -> Response {
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
