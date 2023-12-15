use axum::{extract::{State, Request}, response::{Redirect, IntoResponse, Response}, middleware::Next};
pub use axum::middleware::{from_fn_with_state, from_fn};

use crate::{state::AppState, auth::Session, error};

pub async fn redirect_already_logged_in(_: State<AppState>, session: Result<Session, error::Error>, req: Request, next: Next) -> Response {
    if session.is_ok() {
        return Redirect::to("/").into_response();
    }
    
    next.run(req).await
}

pub async fn assert_is_admin(_: State<AppState>, session: Result<Session, error::Error>, req: Request, next: Next) -> Response {
    if let Ok(session) = session {
        if session.is_admin() {
            return next.run(req).await;
        }
    }
    
    Redirect::to("/").into_response()
}

pub async fn insert_securiy_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut().insert("X-Frame-Options", "DENY".parse().unwrap());
    res.headers_mut().insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    res.headers_mut().insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    res.headers_mut().insert("Referrer-Policy", "no-referrer".parse().unwrap());
    res.headers_mut().insert("Strict-Transport-Security", "max-age=63072000; includeSubDomains".parse().unwrap());
    res
}
