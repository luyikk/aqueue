use async_lock::Mutex;
use std::future::Future;
use std::hint::spin_loop;

/// async future thread safe queue
pub struct AQueue {
    lock: Mutex<()>,
}

impl Default for AQueue {
    #[inline]
    fn default() -> Self {
        AQueue { lock: Mutex::new(()) }
    }
}

impl AQueue {
    #[inline]
    pub fn new() -> AQueue {
        AQueue::default()
    }

    /// Sync run fn
    /// Note: it is not based on fair lock. It will never be called when the queue has unprocessed
    #[inline]
    pub fn sync_run<A, R>(&self, call: impl FnOnce(A) -> R, arg: A) -> R {
        loop {
            let guard = self.lock.try_lock();
            if guard.is_some() {
                return call(arg);
            } else {
                spin_loop();
            }
        }
    }

    /// Async lock run fn
    /// The greatest truths are the simplest
    #[inline]
    pub async fn run<A, T, R>(&self, call: impl FnOnce(A) -> T, arg: A) -> R
    where
        T: Future<Output = R>,
    {
        let _guard = self.lock.lock().await;
        call(arg).await
    }
}
