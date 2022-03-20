use aqueue::Actor;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

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
impl FooRunner for Actor<Foo> {
    async fn add(&self, x: i32) -> i128 {
        self.inner_call(|inner| async move { inner.get_mut().add(x) }).await
    }
    async fn reset(&self) {
        self.inner_call(|inner| async move { inner.get_mut().reset() }).await
    }
    async fn get(&self) -> i128 {
        self.inner_call(|inner| async move { inner.get_mut().get() }).await
    }
    async fn get_count(&self) -> u64 {
        self.inner_call(|inner| async move { inner.get_mut().get_count() }).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    {
        // Single thread test
        let tf = Arc::new(Actor::new(Foo::default()));
        tf.add(100).await;
        assert_eq!(100, tf.get().await);
        tf.add(-100).await;
        assert_eq!(0, tf.get().await);
        tf.reset().await;

        let start = Instant::now();
        for i in 0..2000000 {
            tf.add(i).await;
        }

        println!(
            "test a count:{} value:{} time:{} qps:{}",
            tf.get_count().await,
            tf.get().await,
            start.elapsed().as_secs_f32(),
            tf.get_count().await / start.elapsed().as_millis() as u64 * 1000
        );
    }

    {
        //Multithreading test
        let tf = Arc::new(Actor::new(Foo::default()));
        let start = Instant::now();
        let a_tf = tf.clone();
        let a = tokio::spawn(async move {
            for i in 0..1000000 {
                a_tf.add(i).await;
            }
        });

        let b_tf = tf.clone();
        let b = tokio::spawn(async move {
            for i in 1000000..2000000 {
                b_tf.add(i).await;
            }
        });

        let c_tf = tf.clone();
        let c = tokio::spawn(async move {
            for i in 2000000..3000000 {
                c_tf.add(i).await;
            }
        });

        c.await?;
        a.await?;
        b.await?;

        println!(
            "test b count:{} value:{} time:{} qps:{}",
            tf.get_count().await,
            tf.get().await,
            start.elapsed().as_secs_f32(),
            tf.get_count().await / start.elapsed().as_millis() as u64 * 1000
        );
    }

    Ok(())
}
