#![feature(async_closure)]

use sqlx::MySqlPool;
use std::sync::Arc;
use aqueue::Actor;
use sqlx::mysql::MySqlPoolOptions;
use anyhow::*;
use async_trait ::async_trait;

pub struct DataBases{
    pool:MySqlPool
}

unsafe impl Send for DataBases{}
unsafe impl Sync for DataBases{}

impl DataBases{
    pub fn new(mysql_url:&str,mysql_max_connections:u32)->anyhow::Result<Arc<Actor<DataBases>>>{
        let pool=MySqlPoolOptions::new()
            .max_connections(mysql_max_connections)
            .connect_lazy(mysql_url)?;

        Ok(Arc::new(Actor::new(DataBases{
            pool
        })))

    }

    async fn test(&self)->Result<bool> {
        let (r, ): (u32, ) = sqlx::query_as("select 1").fetch_one(&self.pool).await.unwrap();
        Ok(1 == 1)
    }
}

#[async_trait]
pub trait IDatabase{
    async fn test(&self)->Result<bool>;
}

#[async_trait]
impl IDatabase for Actor<DataBases>{
    async fn test(&self) ->Result<bool> {
        self.inner_call(async move|inner|{
             inner.get().test().await
        }).await
    }
}



#[tokio::main]
async fn main() {

}