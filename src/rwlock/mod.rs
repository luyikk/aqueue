use crate::actor::RefInner;
use crate::rw_model::RefMutInner;
use async_lock::RwLock;
use std::future::Future;
use std::thread::yield_now;

/// An async reader-writer lock queue.
///
/// `RwQueue` wraps [`async_lock::RwLock`] and exposes async methods for
/// shared reads ([`read_run`]) and exclusive writes ([`write_run`]).
/// Multiple read closures may execute concurrently; a write closure runs in
/// isolation.
///
/// Synchronous variants ([`sync_read_run`], [`sync_write_run`]) are provided
/// for non-async call sites.  They yield the thread until the lock is free.
///
/// This is the locking primitive used by [`RwModel`].
///
/// [`read_run`]: RwQueue::read_run
/// [`write_run`]: RwQueue::write_run
/// [`sync_read_run`]: RwQueue::sync_read_run
/// [`sync_write_run`]: RwQueue::sync_write_run
/// [`RwModel`]: crate::RwModel
#[derive(Debug)]
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
    /// Create a new, unlocked `RwQueue`.
    #[inline]
    pub fn new() -> RwQueue {
        RwQueue::default()
    }

    /// Run `call(arg)` synchronously under an exclusive write lock.
    ///
    /// **Yields the thread** (via [`std::thread::yield_now`]) until the write
    /// lock is available.  Avoid calling from async tasks; use
    /// [`write_run`] instead.
    ///
    /// [`write_run`]: RwQueue::write_run
    #[inline]
    pub fn sync_write_run<A, R>(&self, call: impl FnOnce(RefMutInner<'_, A>) -> R, arg: RefMutInner<'_, A>) -> R {
        loop {
            let guard = self.lock.try_write();
            if guard.is_some() {
                return call(arg);
            } else {
                yield_now();
            }
        }
    }

    /// Run `call(arg)` synchronously under a shared read lock.
    ///
    /// **Yields the thread** (via [`std::thread::yield_now`]) until the read
    /// lock is available.  Avoid calling from async tasks; use
    /// [`read_run`] instead.
    ///
    /// [`read_run`]: RwQueue::read_run
    #[inline]
    pub fn sync_read_run<A, R>(&self, call: impl FnOnce(A) -> R, arg: A) -> R {
        loop {
            let guard = self.lock.try_read();
            if guard.is_some() {
                return call(arg);
            } else {
                yield_now();
            }
        }
    }

    /// Acquire an exclusive write lock and run `call(arg)` asynchronously.
    ///
    /// The write lock is held for the entire duration of the future.
    /// Concurrent write callers are serialised; no read operations run
    /// concurrently with a write.
    #[inline]
    pub async fn write_run<'a, A, T, R>(&self, call: impl FnOnce(RefMutInner<'a, A>) -> T, arg: &'a mut A) -> R
    where
        T: Future<Output = R>,
    {
        let arg = RefMutInner { value: arg };
        let _guard = self.lock.write().await;
        call(arg).await
    }

    /// Acquire a shared read lock and run `call(arg)` asynchronously.
    ///
    /// Multiple read operations may run concurrently.  A write lock will
    /// block until all active reads complete.
    #[inline]
    pub async fn read_run<'a, A, T, R>(&self, call: impl FnOnce(RefInner<'a, A>) -> T, arg: &'a A) -> R
    where
        T: Future<Output = R>,
    {
        let arg = RefInner { value: arg };
        let _guard = self.lock.read().await;
        call(arg).await
    }
}
