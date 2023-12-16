use axum::{extract::{State, Request}, response::{Redirect, IntoResponse, Response}, middleware::Next};
pub use axum::middleware::{from_fn_with_state, from_fn};
use http::StatusCode;

use crate::{state::Context, auth::Session, error};

pub async fn redirect_already_logged_in(_: State<Context>, session: Result<Session, error::Error>, req: Request, next: Next) -> Response {
    if session.is_ok() {
        redirect(&req, "/")
    } else {
        next.run(req).await
    }
}

pub async fn assert_is_admin(_: State<Context>, session: Result<Session, error::Error>, req: Request, next: Next) -> Response {
    match session {
        Ok(session) if session.is_admin() => next.run(req).await,
        _ => redirect(&req, "/")
    }
}

fn redirect(req: &Request, to: &str) -> Response {
    if req.headers().get("HX-Request").is_some() {
        let (mut parts, body) = StatusCode::OK.into_response().into_parts();

        parts.headers.insert("HX-Redirect", to.parse().expect("Infallible"));

        Response::from_parts(parts, body)
    } else {
        Redirect::to("/").into_response()
    }
}

/// Insert security headers into the response.
///
/// # Panics
///
/// This function should never panic. It panics if it fails to parse any of the headers.
pub async fn insert_securiy_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    
    response.headers_mut().insert("X-Frame-Options", "DENY".parse().expect("Infallible"));
    response.headers_mut().insert("X-XSS-Protection", "1; mode=block".parse().expect("Infallible"));
    response.headers_mut().insert("X-Content-Type-Options", "nosniff".parse().expect("Infallible"));
    response.headers_mut().insert("Referrer-Policy", "no-referrer".parse().expect("Infallible"));
    response.headers_mut().insert("Strict-Transport-Security", "max-age=63072000; includeSubDomains".parse().expect("Infallible"));
    
    response
}
