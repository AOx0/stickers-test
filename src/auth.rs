use axum::{extract::FromRequestParts, async_trait, RequestPartsExt, http::request::Parts};
use axum_extra::extract::PrivateCookieJar;
use crate::pool::SurrealConn;
use crate::state::AppState;
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Session {
    token: String,
    user_id: String,
    is_admin: bool
}

impl Session {
    pub async fn new(token: String, user_id: String, is_admin: bool, db: SurrealConn) -> Result<Session, Error> {
        match db.authenticate(&token).await {
            Ok(_) => (),
            Err(e) => {
                println!("Auth error: {:?}", e);
                return Err(Error::AuthFailed);
            }
        }

        Ok(Session {
            is_admin,
            user_id,
            token,
        })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    pub fn is_some_admin(s: Option<Session>) -> bool {
        match s {
            Some(s) => s.is_admin(),
            None => false
        }
    }
}




#[async_trait]
impl FromRequestParts<AppState> for Session
{
    type Rejection = Error;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = match parts.extract_with_state::<PrivateCookieJar, AppState>(state).await {
            Ok(jar) => jar,
            Err(e) => {
                println!("Auth error: {:?}", e);
                return Err(Error::AuthFailed);
            }
        };
        
        let token = jar.get("token").ok_or(Error::AuthNoToken)?.value().to_string();
        let is_admin = jar.get("is_admin").ok_or(Error::AuthNoToken)?.value().parse::<bool>().unwrap();
        let user_id = jar.get("user_id").ok_or(Error::AuthNoToken)?.value().to_string();

        Ok(Session::new(token, user_id, is_admin, state.surreal.get().await.unwrap()).await.unwrap())
    }
}