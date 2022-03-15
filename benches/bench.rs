use anyhow::Result;
use aqueue::Actor;
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::join;

#[derive(Default,Debug)]
struct TestBench {
    i: usize
}

impl TestBench {
    #[inline]
    fn add(&mut self,i:usize){
        self.i+=i;
    }

    #[inline]
    fn clean(&mut self){
        self.i=0;
    }
}

#[async_trait::async_trait]
trait ITestBench{
    async fn add(&self,i:usize)->Result<()>;
    async fn clean(&self)->Result<()>;
    fn get(&self)->usize;
}

#[async_trait::async_trait]
impl ITestBench for Actor<TestBench> {
    #[inline]
    async fn add(&self, i: usize) -> Result<()> {
        self.inner_call(|inner|async move{
            inner.get_mut().add(i);
            Ok(())
        }).await
    }

    #[inline]
    async fn clean(&self) -> Result<()> {
        self.inner_call(|inner|async move{
            inner.get_mut().clean();
            Ok(())
        }).await
    }

    #[inline]
    fn get(&self) -> usize {
        unsafe{
            self.deref_inner().i
        }
    }
}

lazy_static::lazy_static!{
    static ref BENCH_DATA:Actor<TestBench>={
        Actor::new(TestBench::default())
    };
}

fn benchmark(c: &mut Criterion) {
    let size: usize = 1000000;
    c.bench_with_input(BenchmarkId::new("single_task_test", size), &size, |b, &s| {
        // Insert a call to `to_async` to convert the bencher to async mode.
        // The timing loops are the same as with the normal bencher.
        b.to_async(tokio::runtime::Builder::new_current_thread().build().unwrap()).iter(|| single_task_test(s));
    });

    println!("single_task_test all:{}",BENCH_DATA.get());

    c.bench_with_input(BenchmarkId::new("multi_task_test", size), &size, |b, &s| {
        // Insert a call to `to_async` to convert the bencher to async mode.
        // The timing loops are the same as with the normal bencher.
        b.to_async(tokio::runtime::Builder::new_multi_thread().build().unwrap()).iter(|| multi_task_test(s/2));
    });

    println!("multi_task_test all:{}",BENCH_DATA.get());
}

async fn single_task_test(size: usize){
    BENCH_DATA.clean().await.unwrap();
    for i in 0..size {
        BENCH_DATA.add(i).await.unwrap();
    }
}

async fn multi_task_test(size: usize){
    BENCH_DATA.clean().await.unwrap();
    let a=tokio::spawn(async move{
        for i in 0..size {
            BENCH_DATA.add(i).await.unwrap();
        }
    });

    let b=tokio::spawn(async move{
        for i in 0..size {
            BENCH_DATA.add(i).await.unwrap();
        }
    });

    let _=join!(a,b);
}



criterion_group!(benches, benchmark);
criterion_main!(benches);
