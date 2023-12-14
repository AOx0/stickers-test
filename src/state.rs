use core::ops::Deref;
use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use std::sync::Arc;
use crate::pool::SurrealPool;

#[derive(Debug, Clone)]
pub struct State {
    pub surreal: SurrealPool,
    pub img_server: String,
    key: Key
}

#[derive(Clone)]
pub struct AppState(Arc<State>);

impl AppState {
    pub fn new<'a>(surreal: SurrealPool, img_server: &'a str) -> Self {
        Self(Arc::new(State {
            img_server: img_server.to_string(),
            surreal,
            key: Key::generate()
        }))
    }
}

// deref so you can still access the inner fields easily
impl Deref for AppState {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}



impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.0.key.clone()
    }
}