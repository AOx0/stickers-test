use deadpool::managed::{self, Pool};
use deadpool::async_trait;
use surrealdb::{Surreal, engine::remote::ws::{Client, Ws}};

#[derive(Debug)]
pub struct Manager {
    url: String
}

pub type SurrealManager = Pool<Manager>;
pub type SurrealConnection = managed::Object<Manager>;

impl Manager {
    /// Create a new `Manager` that handles creating and recyling connections from a 
    /// pool to a `SurrealDB` instance.
    ///
    /// # Panics
    ///
    /// Panics if the runtime cannot be initialized.
    #[must_use]
    pub fn new(url: &str, size: usize) -> managed::Pool<Manager> {
        Pool::builder(Manager { url: url.to_string() })
            .max_size(size)
            .build()
            .expect("No runtime (tokio/async-std) specified")
    }
}

#[async_trait]
impl managed::Manager for Manager {
    type Error = crate::error::Error;
    type Type = Surreal<Client>;

    async fn create(&self) ->  Result<Self::Type, Self::Error> {
        let db = Surreal::new::<Ws>(self.url.as_str()).await?;

        db.use_ns("demo").use_db("demo").await?;

        Ok(db)
    }

    async fn recycle(&self, conn: &mut Self::Type, _: &managed::Metrics) -> managed::RecycleResult<Self::Error> {

        conn.invalidate().await.map_err(Self::Error::from)?;
        conn.use_ns("demo").use_db("demo").await.map_err(Self::Error::from)?;

        Ok(())
    }
}
