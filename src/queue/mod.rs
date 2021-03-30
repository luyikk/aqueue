mod item;
use crate::AResult;
use aqueue_trait::async_trait;
use async_oneshot::Receiver;
use concurrent_queue::ConcurrentQueue;
pub use item::AQueueItem;
use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::hint::spin_loop;

#[async_trait]
pub trait QueueItem {
    async fn run(&self) -> AResult<()>;
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

impl AQueue {
    pub fn new() -> AQueue {
        AQueue {
            deque: ConcurrentQueue::unbounded(),
            state: AtomicU8::new(IDLE),
            lock:AtomicU8::new(IDLE)
        }
    }

    #[inline]
    pub async fn run<A, T, S>(&self, call: impl FnOnce(A) -> T + Send + Sync + 'static, arg: A) -> AResult<S>
    where
        T: Future<Output = AResult<S>> + Send + Sync + 'static,
        S: 'static,
        A: Send + Sync + 'static, {
        self.push(AQueueItem::new(call, arg)).await
    }

    #[inline]
    pub async fn push<T>(&self, (rx, item): (Receiver<AResult<T>>, Box<dyn QueueItem + Send + Sync>)) -> AResult<T> {

        if let Err(er) = self.deque.push(item) {
            return Err(er.to_string().into());
        }

        while self.lock.load(Ordering::Relaxed)==OPEN {
            spin_loop();
        }

        self.run_ing().await?;
        match rx.await {
            Ok(x) => Ok(x?),
            Err(_) => Err("CLOSE".into()),
        }
    }

    #[inline]
    pub async fn run_ing(&self) -> AResult<()> {
        if self.state.compare_exchange(IDLE, OPEN, Ordering::Acquire, Ordering::Acquire) == Ok(IDLE) {
            'recv: loop {
                let item = {
                    self.lock.store(OPEN,Ordering::Release);
                    match self.deque.pop() {
                        Ok(p) => p,
                        _ => {
                            break 'recv;
                        }
                    }
                };
                self.lock.store(IDLE,Ordering::Release);
                item.run().await?;
            }


            self.state.store(IDLE, Ordering::Release);
            self.lock.store(IDLE,Ordering::Release);
        }

        Ok(())
    }
}
