use deadpool::managed::{Manager, self, Pool};
use deadpool::async_trait;
use surrealdb::{Surreal, engine::remote::ws::{Client, Ws}};

#[derive(Debug)]
pub struct SPool {
    url: String
}

pub type SurrealPool = Pool<SPool>;
pub type SurrealConn = managed::Object<SPool>;

impl SPool {
    pub fn new(url: &str, size: usize) -> managed::Pool<SPool> {
        Pool::builder(SPool { url: url.to_string() })
            .max_size(size)
            .build()
            .unwrap()
    }
}

#[async_trait]
impl Manager for SPool {
    type Error = std::io::Error;
    type Type = Surreal<Client>;

    async fn create(&self) ->  Result<Self::Type, Self::Error> {
        let db = Surreal::new::<Ws>(self.url.as_str()).await.unwrap();

        db.use_ns("demo").use_db("demo").await.unwrap();

        Ok(db)
    }

    async fn recycle(&self, conn: &mut Self::Type, _: &managed::Metrics) -> managed::RecycleResult<Self::Error> {

        conn.invalidate().await.unwrap();

        conn.use_ns("demo").use_db("demo").await.unwrap();

        Ok(())
    }
}