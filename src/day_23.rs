use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use maud::html;

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
