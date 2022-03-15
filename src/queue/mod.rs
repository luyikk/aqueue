mod item;

use crate::queue::item::QueueItem;
use anyhow::{anyhow, Result};
use async_oneshot::Receiver;
use concurrent_queue::ConcurrentQueue;
use std::future::Future;
use std::hint::spin_loop;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};

const IDLE: u8 = 0;
const OPEN: u8 = 1;

/// dyn future item trait
trait IQueueItem: Future<Output = Result<()>> {}
/// run future item
type BoxPinQueueItemFuture = Pin<Box<dyn IQueueItem + Send>>;

/// async future thread safe queue
pub struct AQueue {
    deque: ConcurrentQueue<BoxPinQueueItemFuture>,
    state: AtomicU8,
    lock: AtomicU8,
}

impl Default for AQueue {
    #[inline]
    fn default() -> Self {
        AQueue {
            deque: ConcurrentQueue::unbounded(),
            state: AtomicU8::new(IDLE),
            lock: AtomicU8::new(IDLE),
        }
    }
}

impl AQueue {
    #[inline]
    pub fn new() -> AQueue {
        AQueue::default()
    }

    #[inline]
    pub async fn run<A, T, S>(&self, call: impl FnOnce(A) -> T, arg: A) -> Result<S>
    where
        T: Future<Output = Result<S>> + Send + 'static,
        S: Sync + Send + 'static,
        A: Sync + Send + 'static, {
        let (result_receiver, item) = QueueItem::new(call(arg));
        self.push(result_receiver, Box::pin(item)).await
    }

    /// # Safety
    /// 捕获闭包的借用参数，因为通过指针转换,可能会导致自引用问题，请注意
    #[inline]
    pub async unsafe fn ref_run<A, T, S>(&self, call: impl FnOnce(A) -> T, arg: A) -> Result<S>
    where
        T: Future<Output = Result<S>> + Send,
        S: Sync + Send + 'static,
        A: Sync + Send + 'static, {
        type BoxPinBoxFutureLocal<'a> = Box<Pin<Box<dyn IQueueItem + Send + 'a>>>;
        let (result_receiver, item): (Receiver<Result<S>>, BoxPinBoxFutureLocal<'_>) = {
            let (result_receiver, item) = QueueItem::new(call(arg));
            (result_receiver, Box::new(Box::pin(item)))
        };

        let item = Box::from_raw(std::mem::transmute(Box::into_raw(item)));
        self.push(result_receiver, *item).await
    }

    #[inline]
    async fn push<S>(&self, result_receiver: Receiver<Result<S>>, item: BoxPinQueueItemFuture) -> Result<S> {
        self.deque.push(item).map_err(|err| anyhow!(err.to_string()))?;

        while self.lock.load(Ordering::Relaxed) == OPEN {
            spin_loop();
        }

        self.run_ing().await?;
        result_receiver.await.map_err(|_| anyhow!("tx is close"))?
    }

    #[inline]
    async fn run_ing(&self) -> Result<()> {
        if self.state.compare_exchange(IDLE, OPEN, Ordering::Acquire, Ordering::Acquire) == Ok(IDLE) {
            'recv: loop {
                let item = {
                    self.lock.store(OPEN, Ordering::Release);
                    match self.deque.pop() {
                        Ok(item) => item,
                        _ => break 'recv,
                    }
                };
                self.lock.store(IDLE, Ordering::Release);
                item.await?;
            }

            self.state.store(IDLE, Ordering::Release);
            self.lock.store(IDLE, Ordering::Release);
        }
        Ok(())
    }
}
