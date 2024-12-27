use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::html;

pub async fn star() -> Response {
    html! {
        div #star .lit {  }
    }
    .into_string()
    .into_response()
}
