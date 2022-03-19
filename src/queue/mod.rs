use anyhow::Result;
use std::future::Future;
use async_lock::Semaphore;

/// async future thread safe queue
pub struct AQueue {
    lock: async_lock::Semaphore,
}

impl Default for AQueue {
    #[inline]
    fn default() -> Self {
        AQueue {
            lock: Semaphore::new(1),
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
        T: Future<Output = Result<S>>,
    {
        self.check_run(call(arg)).await
    }

    /// The greatest truths are the simplest
    #[inline]
    async fn check_run<S, T>(&self, future: T) -> Result<S>
    where
        T: Future<Output = Result<S>>,
    {
        let guard =  self.lock.acquire().await;
        let r=future.await;
        drop(guard);
        r
    }
}
