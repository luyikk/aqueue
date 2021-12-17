#![feature(async_closure)]
use sqlx::{SqlitePool};
use aqueue::Actor;
use sqlx::sqlite::SqlitePoolOptions;
use anyhow::{anyhow, Context, Result};
use async_trait ::async_trait;
use std::env;
use tokio::task::JoinHandle;

#[derive(sqlx::FromRow,Debug)]
pub struct User { id: i64, name: String, gold:f64 }

pub struct DataBases{
    auto_id:u32,
    pool:SqlitePool
}
unsafe impl Send for DataBases{}
unsafe impl Sync for DataBases{}
impl DataBases{
    pub fn new(sqlite_max_connections:u32)->anyhow::Result<Actor<DataBases>>{
        let pool=SqlitePoolOptions::new()
            .max_connections(sqlite_max_connections)
            .connect_lazy(&env::var("DATABASE_URL")?)?;

        Ok(Actor::new(DataBases{
            auto_id:0,
            pool
        }))

    }
    async fn create_table(&self)->Result<()> {
        sqlx::query(include_str!("table.sql")).execute(&self.pool)
             .await?;
        Ok(())
    }
    async fn insert_user(&mut self,name:&str,gold:f64)->Result<bool> {
        self.auto_id+=1;
        let row = sqlx::query(r#"
            insert into `user`(`id`,`name`,`gold`)
            values(?,?,?)
         "#).bind(&self.auto_id)
            .bind(name)
            .bind(gold)
            .execute(&self.pool)
            .await?
            .rows_affected();

        Ok(row == 1)
    }
    async fn select_all_users(&self)->Result<Vec<User>>{
       Ok(sqlx::query_as::<_,User>("select * from `user` order by name").fetch_all(&self.pool).await?)
    }
}

#[async_trait]
pub trait IDatabase{
    async fn create_table(&self)->Result<()>;
    async fn insert_user<'a>(&'a self, user: &'a str,gold:f64)->Result<bool>;
    async fn select_all_users(&self)->Result<Vec<User>>;
}

#[async_trait]
impl IDatabase for Actor<DataBases>{
    async fn create_table(&self) ->Result<()> {
        self.inner_call(async move|inner|{
             inner.get().create_table().await
        }).await
    }

    async fn insert_user<'a>(&'a self, user: &'a str, gold: f64) -> Result<bool> {
        unsafe {
            self.inner_call_ref(async move |inner| {
                inner.get_mut().insert_user(&user, gold).await
            }).await
        }
    }

    async fn select_all_users(&self) -> Result<Vec<User>> {
        unsafe{
            self.deref_inner().select_all_users().await
        }
    }
}

lazy_static::lazy_static!{
    static ref DB:Actor<DataBases>={
        DataBases::new(50).expect("install db error")
    };
}
#[tokio::main]
async fn main()->Result<()> {
    dotenv::dotenv().ok().context(".env file not found")?;

    DB.create_table().await?;
    let mut join_vec=Vec::with_capacity(100);
    for i in 0..100 {
        let join:JoinHandle<Result<()>>= tokio::spawn(async move {
            for j in 0..10 {
                DB.insert_user(&i.to_string(),j as f64).await?;
            }
            Ok(())
        });

        join_vec.push(join);
    }
    for join in join_vec {
        join.await??;
    }
    for user in DB.select_all_users().await? {
        println!("{:?}",user);
    }

    Ok(())
}