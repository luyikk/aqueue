# fast speed thread safe async execute model
[![Latest Version](https://img.shields.io/crates/v/aqueue.svg)](https://crates.io/crates/aqueue)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/aqueue)
[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/luyikk/aqueue)](https://rust-reportcard.xuri.me/report/github.com/luyikk/aqueue)
[![Rust CI](https://github.com/luyikk/aqueue/actions/workflows/rust.yml/badge.svg)](https://github.com/luyikk/aqueue/actions/workflows/rust.yml)


##  Example **RwModel**
### Suitable for situations with more reading and less writing
```rust
use aqueue::RwModel;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::try_join;

#[derive(Default)]
struct Foo {
    count: u64,
    i: i128,
}

impl Foo {
    pub fn add(&mut self, x: i32) -> i128 {
        self.count += 1;
        self.i += x as i128;
        self.i
    }
    fn reset(&mut self) {
        self.count = 0;
        self.i = 0;
    }
    pub fn get(&self) -> i128 {
        self.i
    }
    pub fn get_count(&self) -> u64 {
        self.count
    }
}

#[async_trait]
pub trait FooRunner {
    async fn add(&self, x: i32) -> i128;
    async fn reset(&self);
    async fn get(&self) -> i128;
    async fn get_count(&self) -> u64;
}

#[async_trait]
impl FooRunner for RwModel<Foo> {
    async fn add(&self, x: i32) -> i128 {
        self.call_mut(|mut inner| async move { inner.add(x) }).await
    }
    async fn reset(&self) {
        self.call_mut(|mut inner| async move { inner.reset() }).await
    }
    async fn get(&self) -> i128 {
        self.call(|inner| async move { inner.get() }).await
    }
    async fn get_count(&self) -> u64 {
        self.call(|inner| async move { inner.get_count() }).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    {
        // Single thread test
        let tf = RwModel::new(Foo::default());
        tf.add(100).await;
        assert_eq!(100, tf.get().await);
        tf.add(-100).await;
        assert_eq!(0, tf.get().await);
        tf.reset().await;

        let start = Instant::now();
        for i in 0..100000000 {
            tf.add(i).await;
        }

        println!(
            "test rw a count:{} value:{} time:{} qps:{}",
            tf.get_count().await,
            tf.get().await,
            start.elapsed().as_secs_f32(),
            tf.get_count().await / start.elapsed().as_millis() as u64 * 1000
        );
    }

    {
        //Multithreading test
        let tf = Arc::new(RwModel::new(Foo::default()));
        let start = Instant::now();
        let a_tf = tf.clone();
        let a = tokio::spawn(async move {
            for i in 0..25000000 {
                a_tf.add(i).await;
            }
        });

        let b_tf = tf.clone();
        let b = tokio::spawn(async move {
            for i in 25000000..50000000 {
                b_tf.add(i).await;
            }
        });

        let c_tf = tf.clone();
        let c = tokio::spawn(async move {
            for i in 50000000..75000000 {
                c_tf.add(i).await;
            }
        });

        let d_tf = tf.clone();
        let d = tokio::spawn(async move {
            for i in 75000000..100000000 {
                d_tf.add(i).await;
            }
        });

        try_join!(a, b, c, d)?;

        println!(
            "test rw b count:{} value:{} time:{} qps:{}",
            tf.get_count().await,
            tf.get().await,
            start.elapsed().as_secs_f32(),
            tf.get_count().await / start.elapsed().as_millis() as u64 * 1000
        );
    }

    Ok(())
}

```
```shell
test rw a count:100000000 value:4999999950000000 time:5.1791396 qps:19308000
test rw b count:100000000 value:4999999950000000 time:5.293417 qps:18892000
```

## Example **Actor** Database
### Suitable for high-performance environments
### (use Actor Trait and Sqlx Sqlite)

```rust
use anyhow::{anyhow, Result};
use aqueue::{inner_wait, Actor};
use async_trait::async_trait;
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
    pub fn new(sqlite_max_connections: u32) -> Result<Actor<DataBases>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(sqlite_max_connections)
            .connect_lazy("sqlite://:memory:")?;

        Ok(Actor::new(DataBases { auto_id: 0, pool }))
    }
    /// create user table from table.sql
    async fn create_table(&self) -> Result<()> {
        sqlx::query(r#"
            CREATE TABLE "user" (
                "id" integer NOT NULL PRIMARY KEY,
                "name" text,
                "gold" real
            );
        "#).execute(&self.pool).await?;
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

#[async_trait]
pub trait IDatabase {
    /// create user table from table.sql
    async fn create_table(&self) -> Result<()>;
    /// insert user data
    async fn insert_user(&self, name: String, gold: f64) -> Result<bool>;
    /// insert user data
    async fn insert_user_ref_name(&self, name: &str, gold: f64) -> Result<bool>;
    /// select all users table
    async fn select_all_users(&self) -> Result<Vec<User>>;
}

#[async_trait]
impl IDatabase for Actor<DataBases> {
    async fn create_table(&self) -> Result<()> {
        self.inner_call(|inner| async move { inner.get().create_table().await }).await
    }
    async fn insert_user(&self, name: String, gold: f64) -> Result<bool> {
        self.inner_call(|inner| async move { inner.get_mut().insert_user(&name, gold).await })
            .await
    }
    async fn insert_user_ref_name(&self, name: &str, gold: f64) -> Result<bool> {
        self.inner_call(|inner| async move { inner.get_mut().insert_user(name, gold).await })
            .await
    }
    async fn select_all_users(&self) -> Result<Vec<User>> {
        unsafe {         
            self.deref_inner().select_all_users().await
        }
    }
}

lazy_static::lazy_static! {
    /// default global static database actor obj
    static ref DB:Actor<DataBases>={
        DataBases::new(50).expect("install db error")
    };
}

#[tokio::main]
async fn main() -> Result<()> {
  
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
    
    Ok(())
}

```

```shell
User { id: 1, name: "0", gold: 0.0 }
User { id: 2, name: "0", gold: 0.0 }
User { id: 3, name: "0", gold: 0.0 }
User { id: 4, name: "10", gold: 0.0 }
User { id: 5, name: "10", gold: 0.0 }
User { id: 6, name: "16", gold: 0.0 }
User { id: 7, name: "10", gold: 0.0 }
...
User { id: 99996, name: "2", gold: 999.0 }
User { id: 99997, name: "8", gold: 999.0 }
User { id: 99998, name: "5", gold: 999.0 }
User { id: 99999, name: "9", gold: 999.0 }
User { id: 100000, name: "10", gold: 999.0 }
```

