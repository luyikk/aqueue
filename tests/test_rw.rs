use anyhow::Result;
use aqueue::{RwModel, RwQueue};

use futures_util::try_join;
use std::cell::Cell;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{sleep, Duration};

static mut VALUE: u64 = 0;

#[tokio::test]
async fn test_base() -> Result<()> {
    let queue = Arc::new(RwQueue::new());

    let a_queue = queue.clone();
    tokio::spawn(async move {
        let x = a_queue
            .write_run(
                |_| async move {
                    println!("a");
                    //delay_for(Duration::from_secs(1)).await;
                    1
                },
                &mut (),
            )
            .await;

        println!("{:?}", x);
    })
    .await?;

    let a_queue = queue.clone();
    tokio::spawn(async move {
        for i in 0..100 {
            a_queue
                .read_run(
                    |_| async move {
                        println!("b:{}", i);
                        ()
                    },
                    &(),
                )
                .await;
        }
    })
    .await?;

    sleep(Duration::from_secs(2)).await;

    let start = Instant::now();
    let mut v = 0u64;
    for mut i in 0..10000000 {
        v = queue
            .write_run(
                |x| async move {
                    unsafe {
                        VALUE += *x;
                        VALUE
                    }
                },
                &mut i,
            )
            .await;
    }

    println!("{} {}", start.elapsed().as_secs_f32(), v);

    assert_eq!(v, 49999995000000);

    Ok(())
}

#[tokio::test]
async fn test_struct() -> Result<()> {
    #[async_trait::async_trait]
    trait IFoo {
        async fn run(&self, x: i32, y: i32) -> i32;
        fn get_count(&self) -> i32;
    }
    pub struct Foo {
        count: Cell<i32>,
    }

    unsafe impl Sync for Foo {}
    #[async_trait::async_trait]
    impl IFoo for Foo {
        async fn run(&self, x: i32, y: i32) -> i32 {
            self.count.set(self.count.get() + 1);
            x + y
        }

        fn get_count(&self) -> i32 {
            self.count.get()
        }
    }

    pub struct MakeActorIFoo {
        inner: Arc<dyn IFoo + Sync + Send>,
        queue: RwQueue,
    }

    impl MakeActorIFoo {
        pub fn from(x: Foo) -> MakeActorIFoo {
            MakeActorIFoo {
                inner: Arc::new(x),
                queue: RwQueue::new(),
            }
        }
    }

    #[async_trait::async_trait]
    impl IFoo for MakeActorIFoo {
        async fn run(&self, x: i32, y: i32) -> i32 {
            self.queue
                .read_run(|inner| async move { inner.run(x, y).await }, &self.inner.clone())
                .await
        }

        fn get_count(&self) -> i32 {
            self.inner.get_count()
        }
    }

    let foo = Foo { count: Cell::new(0) };
    let make = Arc::new(MakeActorIFoo::from(foo));
    let x = make.run(1, 2).await;
    assert_eq!(x, 3);

    let begin = Instant::now();
    let a_make = make.clone();
    let a = tokio::spawn(async move {
        let start = Instant::now();
        for i in 0..2000000 {
            a_make.run(i, i).await;
        }

        println!("a {} {}", start.elapsed().as_secs_f32(), a_make.inner.get_count());
    });

    let b_make = make.clone();
    let b = tokio::spawn(async move {
        let start = Instant::now();
        for i in 0..2000000 {
            b_make.run(i, i).await;
        }

        println!("b {} {}", start.elapsed().as_secs_f32(), b_make.inner.get_count());
    });

    let c = tokio::spawn(async move {
        let start = Instant::now();
        for i in 0..2000000 {
            make.run(i, i).await;
        }
        println!("c {} {}", start.elapsed().as_secs_f32(), make.inner.get_count());
    });

    try_join!(a, b, c)?;

    println!("all secs:{}", begin.elapsed().as_secs_f32());

    Ok(())
}

#[tokio::test]
async fn test_count() -> Result<()> {
    struct Foo {
        count: u64,
        data: String,
    }

    impl Foo {
        pub fn add_one(&mut self) {
            self.count += 1;
            self.data.push_str(&self.count.to_string())
        }

        pub fn get_str(&self) -> String {
            self.data.clone()
        }
    }

    trait IFoo {
        async fn add_one(&self) -> Result<()>;
        async fn get_str(&self) -> Result<String>;
    }

    impl IFoo for RwModel<Foo> {
        async fn add_one(&self) -> Result<()> {
            self.call_mut(|mut inner| async move {
                inner.add_one();
                Ok(())
            })
            .await
        }

        async fn get_str(&self) -> Result<String> {
            self.call(|inner| async move { Ok(inner.get_str()) }).await
        }
    }

    let obj = Arc::new(RwModel::new(Foo {
        count: 0,
        data: "".to_string(),
    }));

    let mut vec = vec![];
    for _ in 0..1000 {
        let p_obj = obj.clone();
        vec.push(tokio::spawn(async move {
            for _ in 0..1000 {
                p_obj.add_one().await.unwrap();
            }
        }));
    }

    for j in vec {
        j.await?;
    }

    let mut check = Foo {
        count: 0,
        data: "".to_string(),
    };

    for _ in 0..1000000 {
        check.add_one();
    }

    let str = obj.get_str().await?;
    assert_eq!(str, check.get_str());

    Ok(())
}

#[tokio::test]
async fn test_string() -> Result<()> {
    let queue = Arc::new(RwQueue::new());
    let str = 12345.to_string();
    let len = queue.read_run(|x| async move { x.len() }, &str).await;
    assert_eq!(len, 5);
    struct Foo {
        i: i32,
    }
    let foo = Foo { i: 5 };
    let len = queue.read_run(|x| async move { x.i }, &foo).await;
    assert_eq!(len, 5);

    Ok(())
}

#[tokio::test]
async fn test_actor() -> Result<()> {
    #[derive(Default)]
    struct Foo {
        i: i32,
        x: i32,
        y: i32,
    }

    impl Foo {
        pub fn get(&self) -> (i32, i32, i32) {
            (self.i, self.x, self.y)
        }
        pub async fn set(&mut self, x: i32, y: i32) -> i32 {
            self.x += x;
            self.y += y;
            sleep(Duration::from_millis(1)).await;
            println!("{} {}", self.x, self.y);
            self.i += 1;
            self.i
        }
    }

    trait FooRunner {
        async fn set(&self, x: i32, y: i32) -> i32;
        async fn get(&self) -> (i32, i32, i32);
        async fn get_len<'a>(&'a self, b: &'a [u8]) -> usize;
    }

    impl FooRunner for RwModel<Foo> {
        async fn set(&self, x: i32, y: i32) -> i32 {
            self.call_mut(|mut inner| async move { inner.set(x, y).await }).await
        }

        async fn get(&self) -> (i32, i32, i32) {
            self.call(|inner| async move { inner.get() }).await
        }

        async fn get_len<'a>(&'a self, b: &'a [u8]) -> usize {
            self.call(|_| async move { b.len() }).await
        }
    }

    let a_foo = Arc::new(RwModel::new(Foo::default()));
    let b_foo = a_foo.clone();
    let b = tokio::spawn(async move {
        for i in 0..100 {
            let x = b_foo.set(i - 1, i + 1).await;
            println!("b:{}", x);
        }
    });

    let c_foo = a_foo.clone();
    let c = tokio::spawn(async move {
        for i in 0..100 {
            let x = c_foo.set(i - 1, i + 1).await;
            println!("c:{}", x);
        }
    });

    for i in 200..300 {
        let x = a_foo.set(i - 1, i + 1).await;
        println!("a:{}", x);
    }

    try_join!(b, c)?;

    assert_eq!((300, 34550, 35150), a_foo.get().await);

    let buff = vec![1, 2, 3, 4, 5];
    let x = { a_foo.get_len(&buff[..]).await };
    assert_eq!(buff.len(), x);

    Ok(())
}

#[tokio::test]
async fn test_rw_model() -> Result<()> {
    struct Foo {
        i: i32,
    }

    impl Foo {
        async fn get(&self) -> i32 {
            println!("get:{}", self.i);
            tokio::time::sleep(Duration::from_secs(1)).await;
            self.i
        }

        async fn set(&mut self, i: i32) {
            println!("set:{}", i);
            self.i = i;
        }
    }

    trait IFoo {
        async fn get(&self) -> i32;
        async fn set(&self, i: i32);
    }

    impl IFoo for RwModel<Foo> {
        async fn get(&self) -> i32 {
            self.call(|inner| async move { inner.get().await }).await
        }

        async fn set(&self, i: i32) {
            self.call_mut(|mut inner| async move { inner.set(i).await }).await
        }
    }

    let foo = Arc::new(RwModel::new(Foo { i: 0 }));

    let mut joins = Vec::new();
    for i in 0..30 {
        //Theoretically, it should be completed in 1 second
        //But adding sleep resulted in a long execution time
        //So RWModel is still only suitable for environments with more read and less write.
        tokio::time::sleep(Duration::from_millis(1)).await;
        let foo = foo.clone();
        joins.push(tokio::spawn(async move {
            foo.get().await;
            foo.set(i).await;
        }))
    }

    for join in joins {
        join.await?;
    }

    Ok(())
}
