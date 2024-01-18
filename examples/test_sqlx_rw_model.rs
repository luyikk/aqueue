use anyhow::{anyhow, Result};
use aqueue::{call_wait, RwModel};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::env;
use tokio::task::JoinHandle;

#[derive(sqlx::FromRow, Debug)]
#[allow(dead_code)]
pub struct User {
    id: i64,
    name: String,
    gold: f64,
}

pub struct DataBases {
    auto_id: u32,
    pool: SqlitePool,
}

unsafe impl Send for DataBases {}
unsafe impl Sync for DataBases {}

impl DataBases {
    pub fn new(sqlite_max_connections: u32) -> Result<RwModel<DataBases>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(sqlite_max_connections)
            .connect_lazy(&env::var("DATABASE_URL")?)?;

        Ok(RwModel::new(DataBases { auto_id: 0, pool }))
    }
    /// create user table from table.sql
    async fn create_table(&self) -> Result<()> {
        sqlx::query(include_str!("table.sql")).execute(&self.pool).await?;
        Ok(())
    }
    /// insert user data
    async fn insert_user(&mut self, name: &str, gold: f64) -> Result<bool> {
        self.auto_id += 1;
        let row = sqlx::query(
            r#"
            insert into `user`(`id`,`name`,`gold`)
            values(?,?,?)
         "#,
        )
        .bind(&self.auto_id)
        .bind(name)
        .bind(gold)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(row == 1)
    }
    /// insert user data
    async fn select_all_users(&self) -> Result<Vec<User>> {
        Ok(sqlx::query_as::<_, User>("select * from `user`").fetch_all(&self.pool).await?)
    }
}

pub(crate) trait IDatabase {
    /// create user table from table.sql
    async fn create_table(&self) -> Result<()>;
    /// insert user data
    async fn insert_user(&self, name: String, gold: f64) -> Result<bool>;
    /// insert user data
    async fn insert_user_ref_name(&self, name: &str, gold: f64) -> Result<bool>;
    /// select all users table
    async fn select_all_users(&self) -> Result<Vec<User>>;
    /// ERROR example
    /// call test_unsafe_block thread blocking
    /// i use timeout to block unlimited thread blocking
    ///
    ///
    ///       call DB test_unsafe_blocking
    ///  ────────────────────┐
    ///                      │
    ///                      ▼
    ///      ┌───────────────▼────────────┐
    ///      │    inner call lock current │
    ///  ┌──►│         thread             │
    ///  │   └───────────────┬────────────┘
    ///  │                   │
    ///  │                   ▼
    ///  │ call insert_user will lock the current thread again
    ///  │      current thread unlimited blocking
    ///  │                   │
    ///  └───────────────────┘
    ///
    async fn test_unsafe_blocking(&self, name: &str, gold: f64) -> Result<bool>;
}

impl IDatabase for RwModel<DataBases> {
    async fn create_table(&self) -> Result<()> {
        self.call_mut(|inner| async move { inner.create_table().await }).await
    }
    async fn insert_user(&self, name: String, gold: f64) -> Result<bool> {
        self.call_mut(|mut inner| async move { inner.insert_user(&name, gold).await }).await
    }
    async fn insert_user_ref_name(&self, name: &str, gold: f64) -> Result<bool> {
        self.call_mut(|mut inner| async move { inner.insert_user(name, gold).await }).await
    }

    async fn select_all_users(&self) -> Result<Vec<User>> {
        self.call(|inner| async move { inner.select_all_users().await }).await
    }

    async fn test_unsafe_blocking(&self, name: &str, gold: f64) -> Result<bool> {
        call_wait!(self, 3000, |_inner| async move { DB.insert_user_ref_name(name, gold).await }).await?
    }
}

lazy_static::lazy_static! {
    /// default global static database actor obj
    static ref DB:RwModel<DataBases>={
        DataBases::new(50).expect("install db error")
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok().ok_or_else(|| anyhow!(".env file not found"))?;
    DB.create_table().await?;
    let mut join_vec = Vec::with_capacity(100);
    // create 100 tokio task run it.
    for i in 0..100 {
        let join: JoinHandle<Result<()>> = tokio::spawn(async move {
            //each task runs 1000 times
            for j in 0..1000 {
                DB.insert_user(i.to_string(), j as f64).await?;
            }
            Ok(())
        });

        join_vec.push(join);
    }
    //wait all task finish
    for join in join_vec {
        join.await??;
    }
    // print all users
    for user in DB.select_all_users().await? {
        println!("{:?}", user);
    }

    DB.test_unsafe_blocking("123123", 1111111f64).await?;

    Ok(())
}
