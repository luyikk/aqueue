# fast speed thread safe async execute queue

# Examples Basic
```rust
use aqueue::AQueue;
static mut VALUE:i32=0;

#[tokio::main]
async fn main()->Result<(),Box<dyn Error+Sync+Send>> {
    let queue = AQueue::new();
    let mut v=0i32;
    for i in 0..2000000 {
        v= queue.run(async move |x| unsafe {
            // thread safe execute
            VALUE += x;
            Ok(VALUE)
        }, i).await?;
    }

    assert_eq!(v,-1455759936);
   
}

```

# Examples Actor Struct
```rust
#![feature(async_closure)]
use aqueue::AQueue;
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
    pub async fn add(&self,x:i32)->Result<i128,Box<dyn Error+ Send + Sync>>{
        self.queue.run(async move |inner| {
            Ok(inner.0.borrow_mut().add(x))
        },self.inner.clone()).await
    }

    pub async fn get(&self)->Result<i128,Box<dyn Error+ Send + Sync>>{
        self.queue.run(async move |inner| {
            Ok(inner.0.borrow().get())
        },self.inner.clone()).await
    }

    pub async fn get_count(&self)->Result<u64,Box<dyn Error+ Send + Sync>>{
        self.queue.run(async move |inner| {
            Ok(inner.0.borrow().get_count())
        },self.inner.clone()).await
    }
}


#[tokio::main]
async fn main() {
    {
        // Single thread test
        let tf = Arc::new(FooRunner::new());
        tf.add(100).await.unwrap();
        assert_eq!(100, tf.get().await.unwrap());
        tf.add(-100).await.unwrap();
        assert_eq!(0, tf.get().await.unwrap());

        let start = Instant::now();
        for i in 0..2000000 {
            if let Err(er) = tf.add(i).await {
                println!("{}", er);
            };
        }

        println!("test a count:{} value:{} time:{} qps:{}",
                 tf.get_count().await.unwrap(),
                 tf.get().await.unwrap(),
                 start.elapsed().as_secs_f32(),
                 tf.get_count().await.unwrap() / start.elapsed().as_millis() as u64 * 1000);
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

        c.await.unwrap();
        a.await.unwrap();
        b.await.unwrap();

        println!("test b count:{} value:{} time:{} qps:{}",
                 tf.get_count().await.unwrap(),
                 tf.get().await.unwrap(),
                 start.elapsed().as_secs_f32(),
                 tf.get_count().await.unwrap() / start.elapsed().as_millis() as u64 * 1000);
    }
}
```

# Examples Actor Trait
```rust
#![feature(async_closure)]
use aqueue::{Actor,AResult};
use std::sync::Arc;
use std::error::Error;
use std::time::Instant;


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

#[aqueue::aqueue_trait]
pub trait FooRunner{
    async fn add(&self,x:i32)->Result<i128,Box<dyn Error+ Send + Sync>>;
    async fn reset(&self)->Result<(),Box<dyn Error+ Send + Sync>>;
    async fn get(&self)->Result<i128,Box<dyn Error+ Send + Sync>>;
    async fn get_count(&self)->Result<u64,Box<dyn Error+ Send + Sync>>;
}

#[aqueue::aqueue_trait]
impl FooRunner for Actor<Foo> {  
    async fn add(&self,x:i32)->Result<i128,Box<dyn Error+ Send + Sync>>{
        self.inner_call(async move |inner|{
            Ok(inner.get_mut().add(x))
        }).await
    }  
    async fn reset(&self)->Result<(),Box<dyn Error+ Send + Sync>>{
        self.inner_call(async move |inner| {
            Ok(inner.get_mut().reset())
        }).await
    }  
    async fn get(&self)->Result<i128,Box<dyn Error+ Send + Sync>>{
        self.inner_call(async move |inner|{
            Ok(inner.get_mut().get())
        }).await
    }  
    async fn get_count(&self)->Result<u64,Box<dyn Error+ Send + Sync>>{
        self.inner_call(async move |inner| {
            Ok(inner.get_mut().get_count())
        }).await
    }
}

#[tokio::main]
async fn main()->Result<(),Box<dyn Error+ Send + Sync>> {
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