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

    /// Async run fn
    /// It is based on the principle of first in, first run
    #[inline]
    pub async fn run<A, T, R>(&self, call: impl FnOnce(A) -> T, arg: A) -> R
    where
        T: Future<Output = R>,
    {
        self.check_run(call(arg)).await
    }

    /// The greatest truths are the simplest
    #[inline]
    async fn check_run<R, T>(&self, future: T) -> R
    where
        T: Future<Output = R>,
    {
        let _guard = self.lock.lock().await;
        future.await
    }
}
