use std::fmt::Display;

use axum::{response::IntoResponse, http::StatusCode};
use deadpool::managed::RecycleError;

#[derive(Debug, Clone)]
pub enum Error {
    AuthNoToken,
    AuthFailed,
    DatabaseError,
    PoolError,
    HyperError,
    HttpError,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       match self {
            Self::AuthNoToken => write!(f, "No token provided"),
            Self::AuthFailed => write!(f, "Authentication failed"),
            Self::DatabaseError => write!(f, "Database error"),
            Self::PoolError => write!(f, "Pool error"),
            Self::HyperError => write!(f, "Hyper error"),
            Self::HttpError => write!(f, "HTTP error"),
       }
    }
}

impl std::error::Error for Error {}

impl Error {
    #[must_use]
    pub fn into_recycle_error(self) -> RecycleError<Self> {
        println!("RecycleError: {self:?}");
        RecycleError::Backend(self)
    }
}

impl From<hyper_util::client::legacy::Error> for Error {
    fn from(e: hyper_util::client::legacy::Error) -> Self {
        println!("Hyper Error: {e:?}");
        Self::HyperError
    }
}

impl From<http::Error> for Error {
    fn from(e: http::Error) -> Self {
        println!("HTTP Error: {e:?}");
        Self::HttpError
    }
}

impl From<deadpool::managed::PoolError<Error>> for Error {
    fn from(e: deadpool::managed::PoolError<Error>) -> Self {
        println!("PoolError: {e:?}");
        Self::PoolError
    }
}

impl From<surrealdb::Error> for Error {
    fn from(e: surrealdb::Error) -> Self {
        println!("SurrealDB Error: {e:?}");
        Self::DatabaseError
    }

}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    }
}