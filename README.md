# fast speed thread safe async execute queue



##  Example **Actor**
```rust
use aqueue::{Actor,AResult};
use std::sync::Arc;
use std::error::Error;
use std::time::Instant;
use async_trait::async_trait;

#[derive(Default)]
struct Foo{
    count:u64,
    i:i128
}

impl Foo{  
    pub fn add(&mut self,x:i32)->i128{
        self.count+=1;
        self.i+=x as i128;
        self.i
    }   
    fn reset(&mut self){
        self.count=0;
        self.i=0;
    }   
    pub fn get(&self)->i128{
        self.i
    }   
    pub fn get_count(&self)->u64{
        self.count
    }
}

#[async_trait]
pub trait FooRunner{
    async fn add(&self,x:i32)->anyhow::Result<i128>;
    async fn reset(&self)->anyhow::Result<()>;
    async fn get(&self)->anyhow::Result<i128>;
    async fn get_count(&self)->anyhow::Result<u64>;
}

#[async_trait]
impl FooRunner for Actor<Foo> {  
    async fn add(&self,x:i32)->anyhow::Result<i128>{
        self.inner_call(|inner| async move {
            Ok(inner.get_mut().add(x))
        }).await
    }  
    async fn reset(&self)->anyhow::Result<()>{
        self.inner_call(|inner| async move  {
            Ok(inner.get_mut().reset())
        }).await
    }  
    async fn get(&self)->anyhow::Result<i128>{
        self.inner_call(|inner| async move {
            Ok(inner.get_mut().get())
        }).await
    }  
    async fn get_count(&self)->anyhow::Result<u64>{
        self.inner_call( |inner| async move {
            Ok(inner.get_mut().get_count())
        }).await
    }
}

#[tokio::main]
async fn main()->anyhow::Result<()> {
    {
        // Single thread test
        let tf = Arc::new(Actor::new(Foo::default()));
        tf.add(100).await?;
        assert_eq!(100, tf.get().await?);
        tf.add(-100).await.unwrap();
        assert_eq!(0, tf.get().await?);
        tf.reset().await?;

        let start = Instant::now();
        for i in 0..2000000 {
            if let Err(er) = tf.add(i).await {
                println!("{}", er);
            };
        }

        println!("test a count:{} value:{} time:{} qps:{}",
                 tf.get_count().await?,
                 tf.get().await?,
                 start.elapsed().as_secs_f32(),
                 tf.get_count().await? / start.elapsed().as_millis() as u64 * 1000);
    }

    {
        //Multithreading test
        let tf = Arc::new(Actor::new(Foo::default()));
        let start = Instant::now();
        let a_tf = tf.clone();
        let a = tokio::spawn(async move {
            for i in 0..1000000 {
                if let Err(er) = a_tf.add(i).await {
                    println!("{}", er);
                };
            }
        });

        let b_tf = tf.clone();
        let b = tokio::spawn(async move {
            for i in 1000000..2000000 {
                if let Err(er) = b_tf.add(i).await {
                    println!("{}", er);
                };
            }
        });

        let c_tf = tf.clone();
        let c = tokio::spawn(async move {
            for i in 2000000..3000000 {
                if let Err(er) = c_tf.add(i).await {
                    println!("{}", er);
                };
            }
        });

        c.await?;
        a.await?;
        b.await?;

        println!("test b count:{} value:{} time:{} qps:{}",
                 tf.get_count().await?,
                 tf.get().await?,
                 start.elapsed().as_secs_f32(),
                 tf.get_count().await? / start.elapsed().as_millis() as u64 * 1000);
    }

    Ok(())
}
```

## Example Database 
### (use Actor Trait and Sqlx Sqlite)

```rust
use sqlx::{SqlitePool};
use aqueue::Actor;
use sqlx::sqlite::SqlitePoolOptions;
use anyhow::*;
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
    /// create user table from table.sql
    async fn create_table(&self)->anyhow::Result<()> {
        sqlx::query(include_str!("table.sql")).execute(&self.pool)
            .await?;
        Ok(())

    }
    /// insert user data
    async fn insert_user(&mut self,name:&str,gold:f64)->anyhow::Result<bool> {
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
    /// insert user data
    async fn select_all_users(&self)->anyhow::Result<Vec<User>>{
        Ok(sqlx::query_as::<_,User>("select * from `user`").fetch_all(&self.pool).await?)
    }
}

#[async_trait]
pub trait IDatabase{
    /// create user table from table.sql
    async fn create_table(&self)->anyhow::Result<()>;
    /// insert user data
    async fn insert_user(&self,name:String,gold:f64)->anyhow::Result<bool>;
    /// insert user data
    async fn insert_user_ref_name(&self,name:&str,gold:f64)->anyhow::Result<bool>;
    /// select all users table
    async fn select_all_users(&self)->anyhow::Result<Vec<User>>;
}

#[async_trait]
impl IDatabase for Actor<DataBases>{
    
    async fn create_table(&self) ->anyhow::Result<()> {
        self.inner_call(|inner| async move{
            inner.get().create_table().await
        }).await
    }

    async fn insert_user(&self, name: String, gold: f64) -> anyhow::Result<bool> {
        self.inner_call(|inner| async move{
            inner.get_mut().insert_user(&name,gold).await
        }).await
    }

    async fn insert_user_ref_name(&self, name: &str, gold: f64) -> anyhow::Result<bool> {
        unsafe {
            // warn: 
            // Don't ref your &self
            self.inner_call_ref(|inner| async move {
                inner.get_mut().insert_user(name, gold).await
            }).await
        }
    }

    async fn select_all_users(&self) -> anyhow::Result<Vec<User>> {
        unsafe{
            // warn:
            // This is a thread unsafe way to get
            // When using, please make sure there is no thread safety problem
            self.deref_inner().select_all_users().await
        }
    }
}

lazy_static::lazy_static!{
    /// default global static database actor obj
    static ref DB:Actor<DataBases>={
        DataBases::new(50).expect("install db error")
    };
}

#[tokio::main]
async fn main()->anyhow::Result<()> {
    dotenv::dotenv().ok().ok_or_else(||anyhow!(".env file not found"))?;

    DB.create_table().await?;
    
    let mut join_vec=Vec::with_capacity(100);
    // create 100 tokio task run it.
    for i in 0..100 {
        let join:JoinHandle<Result<()>>= tokio::spawn(async move {
            //each task runs 1000 times
            for j in 0..1000 {
                DB.insert_user(i.to_string(),j as f64).await?;
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
        println!("{:?}",user);
    }

    Ok(())
}
```

```shell
cargo run --color=always --package aqueue --example test_sqlx --release
    Finished release [optimized] target(s) in 0.21s
     Running `target\release\examples\test_sqlx.exe`
User { id: 1, name: "0", gold: 0.0 }
User { id: 2, name: "1", gold: 0.0 }
User { id: 3, name: "2", gold: 0.0 }
User { id: 4, name: "3", gold: 0.0 }
User { id: 5, name: "4", gold: 0.0 }
User { id: 6, name: "5", gold: 0.0 }
...
User { id: 9996, name: "0", gold: 95.0 }
User { id: 9997, name: "0", gold: 96.0 }
User { id: 9998, name: "0", gold: 97.0 }
User { id: 9999, name: "0", gold: 98.0 }
User { id: 10000, name: "0", gold: 99.0 }
Process finished with exit code 0
```


## Example Basic
```rust
use aqueue::AQueue;
static mut VALUE:i32=0;

#[tokio::main]
async fn main()->Result<(),Box<dyn Error>> {
    let queue = AQueue::new();
    let mut v=0i32;
    for i in 0..2000000 {
        v= queue.run(|x| async move {
            unsafe {
                // thread safe execute
                VALUE += x;
                Ok(VALUE)
            }
        }, i).await?;
    }

    assert_eq!(v,-1455759936);
   
}

```

## Example not used trait actor 
```rust
use aqueue::{AResult,AQueue};
use std::sync::Arc;
use std::cell::{RefCell};
use std::error::Error;
use std::time::Instant;


struct Foo{
    count:u64,
    i:i128
}

impl Foo{
    pub fn add(&mut self,x:i32)->i128{
        self.count+=1;
        self.i+=x as i128;
        self.i
    }

    pub fn get(&self)->i128{
        self.i
    }
    pub fn get_count(&self)->u64{
        self.count
    }
}

struct Store<T>(RefCell<T>);
unsafe impl<T> Sync for Store<T>{}
unsafe impl<T> Send for Store<T>{}

impl<T> Store<T>{
    pub fn new(x:T)->Store<T>{
        Store(RefCell::new(x))
    }
}

struct FooRunner {
    inner:Arc<Store<Foo>>,
    queue:AQueue
}

impl FooRunner {
    pub fn new()-> FooRunner {
        FooRunner {
            inner:Arc::new(Store::new(Foo{ count:0, i:0})),
            queue:AQueue::new()
        }
    }
    pub async fn add(&self,x:i32)->anyhow::Result<i128>{
        self.queue.run(|inner| async move  {
            Ok(inner.0.borrow_mut().add(x))
        },self.inner.clone()).await
    }

    pub async fn get(&self)->anyhow::Result<i128>{
        self.queue.run(|inner| async move  {
            Ok(inner.0.borrow().get())
        },self.inner.clone()).await
    }

    pub async fn get_count(&self)->anyhow::Result<u64>{
        self.queue.run(|inner| async move {
            Ok(inner.0.borrow().get_count())
        },self.inner.clone()).await
    }
}


#[tokio::main]
async fn main()->anyhow::Result<()> {
    {
        // Single thread test
        let tf = Arc::new(FooRunner::new());
        tf.add(100).await?;
        assert_eq!(100, tf.get().await?);
        tf.add(-100).await.unwrap();
        assert_eq!(0, tf.get().await?);

        let start = Instant::now();
        for i in 0..2000000 {
            if let Err(er) = tf.add(i).await {
                println!("{}", er);
            };
        }

        println!("test a count:{} value:{} time:{} qps:{}",
                 tf.get_count().await?,
                 tf.get().await?,
                 start.elapsed().as_secs_f32(),
                 tf.get_count().await? / start.elapsed().as_millis() as u64 * 1000);
    }

    {
        //Multithreading test
        let tf = Arc::new(FooRunner::new());
        let start = Instant::now();
        let a_tf = tf.clone();
        let a = tokio::spawn(async move {
            for i in 0..1000000 {
                if let Err(er) = a_tf.add(i).await {
                    println!("{}", er);
                };
            }
        });

        let b_tf = tf.clone();
        let b = tokio::spawn(async move {
            for i in 1000000..2000000 {
                if let Err(er) = b_tf.add(i).await {
                    println!("{}", er);
                };
            }
        });

        let c_tf = tf.clone();
        let c = tokio::spawn(async move {
            for i in 2000000..3000000 {
                if let Err(er) = c_tf.add(i).await {
                    println!("{}", er);
                };
            }
        });

        c.await?;
        a.await?;
        b.await?;

        println!("test b count:{} value:{} time:{} qps:{}",
                 tf.get_count().await?,
                 tf.get().await?,
                 start.elapsed().as_secs_f32(),
                 tf.get_count().await? / start.elapsed().as_millis() as u64 * 1000);       
    }

    Ok(())
}
```
