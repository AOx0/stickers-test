use core::ops::Deref;
use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use std::sync::Arc;
use crate::pool::SurrealManager;

#[derive(Debug, Clone)]
pub struct State {
    pub surreal: SurrealManager,
    pub img_server: String,
    key: Key
}

#[derive(Clone)]
pub struct Context(Arc<State>);

impl Context {
    #[must_use]
    pub fn new(surreal: SurrealManager, img_server: &str) -> Self {
        Self(Arc::new(State {
            img_server: img_server.to_string(),
            surreal,
            key: Key::generate()
        }))
    }
}

// deref so you can still access the inner fields easily
impl Deref for Context {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}



impl FromRef<Context> for Key {
    fn from_ref(state: &Context) -> Self {
        state.0.key.clone()
    }
}