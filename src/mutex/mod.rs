use async_lock::Mutex;
use std::future::Future;

/// A lightweight async mutex queue.
///
/// `AQueue` wraps [`async_lock::Mutex`] and exposes a single [`run`] method
/// that acquires the lock, executes an async closure, and releases the lock
/// when the future completes.  This guarantees that only one closure runs at
/// a time — the serialisation primitive used by [`Actor`].
///
/// A synchronous variant [`sync_run`] is also provided for non-async call
/// sites; it yields the thread using [`std::thread::yield_now`] until the
/// lock becomes free.
///
/// [`run`]: AQueue::run
/// [`sync_run`]: AQueue::sync_run
/// [`Actor`]: crate::Actor
#[derive(Debug)]
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
    /// Create a new, unlocked `AQueue`.
    #[inline]
    pub fn new() -> AQueue {
        AQueue::default()
    }

    /// Run `call(arg)` synchronously after acquiring the mutex.
    ///
    /// This method **yields the thread** (via [`std::thread::yield_now`]) until
    /// the lock is available.  It must only be called from a context where
    /// blocking the current thread is acceptable (e.g. inside
    /// [`tokio::task::spawn_blocking`]).  Calling it from an async task will
    /// starve other tasks sharing the same executor thread.
    ///
    /// [`tokio::task::spawn_blocking`]: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
    #[inline]
    pub fn sync_run<A, R>(&self, call: impl FnOnce(A) -> R, arg: A) -> R {
        loop {
            let guard = self.lock.try_lock();
            if guard.is_some() {
                return call(arg);
            } else {
                std::thread::yield_now()
            }
        }
    }

    /// Acquire the mutex and run `call(arg)` asynchronously.
    ///
    /// The lock is held for the entire duration of the returned future.
    /// Concurrent callers are queued and execute one at a time in FIFO order.
    #[inline]
    pub async fn run<'a, A, T, R>(&self, call: impl FnOnce(&'a A) -> T, arg: &'a A) -> R
    where
        T: Future<Output = R>,
    {
        let _guard = self.lock.lock().await;
        call(arg).await
    }
}
