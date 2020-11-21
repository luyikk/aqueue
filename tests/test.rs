#![feature(async_closure)]

use aqueue::AQueue;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{delay_for, Duration};

static mut VALUE: i32 = 0;

#[tokio::test]
async fn test() -> Result<(), Box<dyn Error + Sync + Send>> {
    let queue = Arc::new(AQueue::new());

    let a_queue = queue.clone();
    tokio::spawn(async move {
        let x = a_queue
            .run(
                async move |_| {
                    println!("a");
                    delay_for(Duration::from_secs(1)).await;
                    Ok(1)
                },
                (),
            )
            .await;

        println!("{:?}", x);
    });

    let a_queue = queue.clone();
    tokio::spawn(async move {
        for i in 0..100 {
            a_queue
                .run(
                    async move |_| {
                        println!("b:{}", i);
                        Ok(())
                    },
                    (),
                )
                .await
                .unwrap();
        }
    });

    delay_for(Duration::from_secs(2)).await;

    let start = Instant::now();
    let mut v = 0i32;
    for i in 0..2000000 {
        v = queue
            .run(
                async move |x| unsafe {
                    VALUE += x;
                    Ok(VALUE)
                },
                i,
            )
            .await?;
    }

    println!("{} {}", start.elapsed().as_secs_f32(), v);

    assert_eq!(v, -1455759936);

    Ok(())
}

#[tokio::test]
async fn test_string()-> Result<(), Box<dyn Error + Sync + Send>>{


   let queue = Arc::new(AQueue::new());

   let str=12345.to_string();

   let len= queue.run(async move|x|{
        Ok(x.len())
    },str).await?;

    assert_eq!(len,5);


    struct Foo{
        i:i32
    }

    let foo=Foo{i:5};

    let len= queue.run(async move|x|{
        Ok(x.i)
    },foo).await?;

    assert_eq!(len,5);



    Ok(())
}


use aqueue::aqueue_trait;
use std::cell::Cell;
use aqueue::actor::{Actor};
use tokio::task::JoinHandle;


#[tokio::test]
async fn test_struct() {
    #[aqueue_trait]
    pub trait IFoo {
        async fn run(&self, x: i32, y: i32) -> i32;
        fn get_count(&self)->i32;
    }
    pub struct Foo{
        count:Cell<i32>
    }

    unsafe impl  Sync for Foo{};

    #[aqueue_trait]
    impl IFoo for Foo {
        async fn run(&self, x: i32, y: i32) -> i32 {
            self.count.set(self.count.get()+1);
            x + y
        }

        fn get_count(&self)->i32{
            self.count.get()
        }
    }

    pub struct MakeActorIFoo {
        inner: Arc<dyn IFoo + Sync + Send>,
        queue: AQueue,
    }

    impl MakeActorIFoo{
        pub fn from(x:Foo)->MakeActorIFoo{
            MakeActorIFoo{
                inner:Arc::new(x),
                queue:AQueue::new()
            }
        }
    }

    #[aqueue_trait]
    impl IFoo for MakeActorIFoo {
        async fn run(&self, x: i32, y: i32) -> i32 {
            self.queue
                .run(async move |inner|{
                    Ok(inner.run(x, y).await)
                }, self.inner.clone())
                .await
                .unwrap()
        }

        fn get_count(&self) -> i32 {
            self.inner.get_count()
        }
    }



    let foo=Foo{
       count:Cell::new(0)
    };

    let make=Arc::new(MakeActorIFoo::from(foo));

    let x= make.run(1,2).await;

    assert_eq!(x,3);

    let start = Instant::now();
    for i in 0..2000000 {
        make.run(i,i).await;
    }

    println!("{} {}" , start.elapsed().as_secs_f32(),make.inner.get_count());

    let a_make=make.clone();
    let a= tokio::spawn(async move{
        let start = Instant::now();
        for i in 0..2000000 {
            a_make.run(i,i).await;
        }

        println!("a {} {}" , start.elapsed().as_secs_f32(),a_make.inner.get_count());

    });

    let b_make=make.clone();
    let b= tokio::spawn(async move{
        let start = Instant::now();
        for i in 0..2000000 {
            b_make.run(i,i).await;
        }

        println!("b {} {}" , start.elapsed().as_secs_f32(),b_make.inner.get_count());

    });

    a.await.unwrap();
    b.await.unwrap();
}


#[tokio::test]
async fn test_actor()->Result<(),Box<dyn Error+ Send + Sync>>{

    #[derive(Default)]
    struct Foo{
        i:i32,
        x:i32,
        y:i32
    }

    impl Foo{

        pub fn get(&self)->(i32,i32,i32){
            (self.i,self.x,self.y)
        }
        pub async fn set(&mut self,x:i32,y:i32)->i32{
            self.x+=x;
            self.y+=y;
            delay_for(Duration::from_millis(1)).await;
            println!("{} {}",self.x,self.y);
            self.i+=1;
            self.i
        }
    }

    #[aqueue_trait]
    pub trait FooRunner{
        async fn set(&self,x:i32,y:i32)-> Result<i32,Box<dyn Error+Sync+Send>>;
        async fn get(&self)->Result<(i32,i32,i32),Box<dyn Error+Sync+Send>>;
    }

    #[aqueue_trait]
    impl FooRunner for Actor<Foo>{
        async fn set(&self, x: i32, y: i32) -> Result<i32, Box<dyn Error + Sync + Send>> {
            self.inner_call(async move|inner|  {
                Ok(inner.get_mut().set(x,y).await)
            }).await
        }

        async fn get(&self) -> Result<(i32, i32, i32), Box<dyn Error + Sync + Send>> {
            self.inner_call(async move|inner|  {
                Ok(inner.get().get())
            }).await
        }


    }

    let a_foo=Arc::new(Actor::new(Foo::default()));
    let b_foo=a_foo.clone();
    let b:JoinHandle<Result<(),Box<dyn Error+Send+Sync>>>=tokio::spawn(async move{
        for i in 0..100 {
            let x= b_foo.set(i-1,i+1).await?;
            println!("i:{}",x);
        }
        Ok(())
    });

    for i in 200..300 {
        let x= a_foo.set(i-1,i+1).await?;
        println!("i:{}",x);
    }
    b.await??;
    assert_eq!((200,29700,30100),a_foo.get().await?);
    Ok(())
}