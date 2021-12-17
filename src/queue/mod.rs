mod item;
use async_trait::async_trait;
use async_oneshot::Receiver;
use concurrent_queue::ConcurrentQueue;
pub use item::AQueueItem;
use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::hint::spin_loop;
use anyhow::*;

#[async_trait]
pub trait QueueItem {
    async fn run(&self) -> Result<()>;
}

const IDLE: u8 = 0;
const OPEN: u8 = 1;


pub struct AQueue {
    deque: ConcurrentQueue<Box<dyn QueueItem + Send + Sync>>,
    state: AtomicU8,
    lock:AtomicU8
}

unsafe impl Send for AQueue {}
unsafe impl Sync for AQueue {}

impl Default for AQueue{
    fn default() -> Self {
        AQueue {
            deque: ConcurrentQueue::unbounded(),
            state: AtomicU8::new(IDLE),
            lock:AtomicU8::new(IDLE)
        }
    }
}

impl AQueue {
    pub fn new() -> AQueue {
        AQueue::default()
    }

    #[inline]
    pub async fn run<A, T, S>(&self, call: impl FnOnce(A) -> T , arg: A) -> Result<S>
    where
        T: Future<Output = Result<S>> + Send  + 'static,
        S: 'static+Sync+Send,
        A: Send + Sync + 'static, {

        let (rx,item)=AQueueItem::new(Box::pin(call(arg)));
        self.push(rx,Box::new(item)).await
    }

    /// # Safety
    ///
    /// 捕获闭包的借用参数，因为通过指针转换,可能会导致自引用问题，请注意
    #[inline]
    pub async unsafe fn ref_run<'a,A, T, S>(&'a self, call: impl FnOnce(A) -> T , arg: A) -> Result<S>
        where
            T: Future<Output = Result<S>> + Send  + 'a,
            S: 'static+Sync+Send,
            A: Send + Sync + 'static, {

        let (rx,item):(Receiver<Result<S>>,Box<dyn QueueItem + Send + Sync+'a>)={
            let (rx,item)=AQueueItem::new(Box::pin(call(arg)));
            (rx,Box::new(item))
        };

        let item=Box::from_raw(std::mem::transmute(Box::into_raw(item)));
        self.push(rx,item).await
    }

    #[inline]
    pub async fn push<T>(&self, rx:Receiver<Result<T>>, item: Box<dyn QueueItem + Send + Sync>) -> Result<T> {
        self.deque.push(item)
            .map_err(|err| anyhow!(err.to_string()))?;

        while self.lock.load(Ordering::Relaxed) == OPEN {
            spin_loop();
        }

        self.run_ing().await?;
        rx.await.map_err(|_| anyhow!("tx is close"))?
    }

    #[inline]
    pub async fn run_ing(&self) -> Result<()> {
        if self.state.compare_exchange(IDLE, OPEN, Ordering::Acquire, Ordering::Acquire) == Ok(IDLE) {
            'recv:
            loop {
                let item = {
                    self.lock.store(OPEN, Ordering::Release);
                    match self.deque.pop() {
                        Ok(p) => p,
                        _ => break 'recv
                    }
                };
                self.lock.store(IDLE, Ordering::Release);
                item.run().await?;
            }

            self.state.store(IDLE, Ordering::Release);
            self.lock.store(IDLE,Ordering::Release);
        }
        Ok(())
    }
}
