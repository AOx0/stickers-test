use axum::{extract::FromRequestParts, async_trait, RequestPartsExt, http::request::Parts};
use axum_extra::extract::PrivateCookieJar;
use surrealdb::sql::Thing;
use crate::pool::SurrealConn;
use crate::state::AppState;
use crate::error::Error;

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Session {
    #[serde(skip)]
    token: String,
    id: Thing,
    is_admin: bool,
    first_name: String,
    last_name: String,
    email: String,
}

impl Session {
    pub async fn new(token: String, db: SurrealConn) -> Result<Session, Error> {
        match db.authenticate(&token).await {
            Ok(_) => (),
            Err(e) => {
                println!("Auth error: {:?}", e);
                return Err(Error::AuthFailed);
            }
        }
        
        match db.query("SELECT * FROM $auth.id").await {
            Ok(mut res) => {
                let user: Result<Option<Session>, _> = res.take(0);
                match user {
                    Ok(Some(mut user)) => {
                        user.token = token;
                        Ok(user)
                    },
                    Ok(None) => {
                        println!("Auth error: {:?}", Error::AuthFailed);
                        Err(Error::AuthFailed)
                    },
                    Err(e) => {
                        println!("Auth error: {:?}", e);
                        return Err(Error::AuthFailed);
                    }, 
                }
            },
            Err(e) => {
                println!("Auth error: {:?}", e);
                Err(Error::AuthFailed)
            }
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    pub fn id(&self) -> &Thing {
        &self.id
    }

    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    pub fn email(&self) -> &str {
        &self.email
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

        Ok(Session::new(token, state.surreal.get().await.unwrap()).await.unwrap())
    }
}