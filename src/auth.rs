use axum::{extract::FromRequestParts, async_trait, RequestPartsExt, http::request::Parts};
use axum_extra::extract::PrivateCookieJar;
use surrealdb::sql::Thing;
use crate::pool::SurrealConnection;
use crate::state::Context;
use crate::error::Error;

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Session {
    #[serde(skip)]
    token: String,
    id: Thing,
    #[serde(default)]
    is_admin: bool,
    first_name: String,
    last_name: String,
    email: String,
}

impl Session {
    /// Create a new `Session` from a token and a database connection.
    /// 
    /// # Errors
    ///
    /// This function will return an error if the token is invalid or the database is unreachable.
    pub async fn new(token: String, db: SurrealConnection) -> Result<Session, Error> {
        db.authenticate(&token).await?;
        
        let mut res = db.query("SELECT * FROM $auth.id").await?;
 
        let user: Result<Option<Session>, _> = res.take(0);
        match user {
            Ok(Some(mut user)) => {
                user.token = token;
                Ok(user)
            },
            Ok(None) => {
                println!("Auth error: {e:?}", e = Error::AuthFailed);
                Err(Error::AuthFailed)
            },
            Err(e) => {
                println!("Auth error: {e:?}");
                Err(Error::AuthFailed)
            }, 
        }  
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    #[must_use]
    pub fn is_admin(&self) -> bool {
        self.is_admin
    }

    #[must_use]
    pub fn id(&self) -> &Thing {
        &self.id
    }

    #[must_use]
    pub fn first_name(&self) -> &str {
        &self.first_name
    }

    #[must_use]
    pub fn last_name(&self) -> &str {
        &self.last_name
    }

    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }
}




#[async_trait]
impl FromRequestParts<Context> for Session
{
    type Rejection = Error;
    async fn from_request_parts(parts: &mut Parts, state: &Context) -> Result<Self, Self::Rejection> {
        let jar = match parts.extract_with_state::<PrivateCookieJar, Context>(state).await {
            Ok(jar) => jar,
            Err(e) => unreachable!("Failed to extract cookie jar: {:?}", e),
        };
        
        let token = jar.get("token").ok_or(Error::AuthNoToken)?.value().to_string();

        Ok(Session::new(token, state.surreal.get().await?).await?)
    }
}