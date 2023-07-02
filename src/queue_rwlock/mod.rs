use async_lock::RwLock;
use std::future::Future;
use std::hint::spin_loop;

/// async future thread safe queue for Rwlock
pub struct RwQueue {
    lock: RwLock<()>,
}

impl Default for RwQueue {
    #[inline]
    fn default() -> Self {
        RwQueue { lock: RwLock::new(()) }
    }
}

impl RwQueue {
    #[inline]
    pub fn new() -> RwQueue {
        RwQueue::default()
    }

    /// Sync write run fn
    /// Note: it is not based on fair lock. It will never be called when the queue has unprocessed
    #[inline]
    pub fn sync_write_run<A, R>(&self, call: impl FnOnce(A) -> R, arg: A) -> R {
        loop {
            let guard = self.lock.try_write();
            if guard.is_some() {
                return call(arg);
            } else {
                spin_loop();
            }
        }
    }

    /// Sync run fn
    /// Note: it is not based on fair lock. It will never be called when the queue has unprocessed
    #[inline]
    pub fn sync_read_run<A, R>(&self, call: impl FnOnce(A) -> R, arg: A) -> R {
        loop {
            let guard = self.lock.try_read();
            if guard.is_some() {
                return call(arg);
            } else {
                spin_loop();
            }
        }
    }

    /// Async write run fn
    /// It is based on the principle of first in, first run
    #[inline]
    pub async fn write_run<A, T, R>(&self, call: impl FnOnce(A) -> T, arg: A) -> R
    where
        T: Future<Output = R>,
    {
        let _guard = self.lock.write().await;
        call(arg).await
    }

    /// Async write run fn
    /// It is based on the principle of first in, first run
    #[inline]
    pub async fn read_run<A, T, R>(&self, call: impl FnOnce(A) -> T, arg: A) -> R
    where
        T: Future<Output = R>,
    {
        let _guard = self.lock.read().await;
        call(arg).await
    }
}
