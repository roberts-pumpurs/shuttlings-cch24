use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use maud::html;
use serde::Deserialize;
use toml::{map::Map, Value};

pub async fn star() -> Response {
    html! {
        div #star .lit {  }
    }
    .into_string()
    .into_response()
}

pub async fn colour_present(Path(colour): Path<String>) -> Result<Html<String>, StatusCode> {
    let next_colour = match colour.as_str() {
        "red" => "blue",
        "blue" => "purple",
        "purple" => "red",
        _ => return Err(StatusCode::IM_A_TEAPOT),
    };

    let html = html! {
        div ."present" .(colour) hx-get={"/23/present/"(next_colour)""} hx-swap="outerHTML" {
            div .ribbon {  }
            div .ribbon {  }
            div .ribbon {  }
            div .ribbon {  }
        }
    }
    .into_string();

    Ok(Html(html))
}

pub async fn ornament(
    Path((state, n)): Path<(String, String)>,
) -> Result<Html<String>, StatusCode> {
    let (next_state, current_state) = match &*state {
        "on" => ("off", "ornament on"),
        "off" => ("on", "ornament"),
        _ => return Err(StatusCode::IM_A_TEAPOT),
    };

    let html = html! {
        div
            .(current_state)
            id={"ornament"(n)}
            hx-get={"/23/ornament/"(next_state)"/"(n)}
            hx-trigger="load delay:2s once"
            hx-swap="outerHTML" {
        }
    }
    .into_string();

    Ok(Html(html))
}

#[derive(Deserialize)]
struct Package {
    _name: Option<String>,
    _source: Option<String>,
    _version: Option<String>,
    checksum: String,
}
impl Package {
    fn cal(self) -> Option<(String, u8, u8)> {
        // We expect at least 10 hex characters
        if self.checksum.len() < 10 {
            return None;
        }

        let color = &self.checksum[0..6]; // #RRGGBB
        let top_hex = &self.checksum[6..8];
        let left_hex = &self.checksum[8..10];

        // Ensure only valid hex chars in the color portion
        if !color.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        // Parse top and left as hex into u8
        let top = u8::from_str_radix(top_hex, 16).ok()?;
        let left = u8::from_str_radix(left_hex, 16).ok()?;

        Some((format!("#{color}"), top, left))
    }
}

pub async fn lockfile(mut multipart: Multipart) -> Result<Html<String>, StatusCode> {
    let mut htmls = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let data = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;

        let payload: Map<String, Value> =
            toml::from_str(&data).map_err(|_| StatusCode::BAD_REQUEST)?;
        let packages = payload["package"].as_array().unwrap();

        for package in packages {
            if let Ok(payload) = package.clone().try_into::<Package>() {
                let d = payload.cal().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
                htmls.push(d);
            }
        }
    }
    if htmls.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let html = htmls
        .into_iter()
        .map(|(color, top, left)| {
            html! {
             div style={"background-color:"(color)";top:"(top)"px;left:"(left)"px;"} {

             }
            }
            .into_string()
        })
        .collect();

    Ok(Html(html))
}
