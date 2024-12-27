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
