use axum::{response::IntoResponse, http::StatusCode};

#[derive(Debug, Clone)]
pub enum Error {
    AuthNoToken,
    AuthFailed,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}