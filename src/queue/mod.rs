use anyhow::Result;
use std::future::Future;
use async_lock::Mutex;

/// async future thread safe queue
pub struct AQueue {
    lock: async_lock::Mutex<()>,
}

impl Default for AQueue {
    #[inline]
    fn default() -> Self {
        AQueue {
            lock: Mutex::new(()),
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
        let _guard = self.lock.lock().await;
        future.await
    }
}
