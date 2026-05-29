use async_lock::Semaphore;
use std::future::Future;

/// An async semaphore queue that limits the number of concurrently executing
/// closures.
///
/// `SemaphoreQueue` wraps [`async_lock::Semaphore`] and exposes a single
/// [`run`] method.  Each call acquires one permit before executing the
/// closure; the permit is released when the future completes.  If all permits
/// are held, new callers suspend until one becomes available.
///
/// This is the concurrency-control primitive used by [`PCModel`].
///
/// [`run`]: SemaphoreQueue::run
/// [`PCModel`]: crate::PCModel
#[derive(Debug)]
pub struct SemaphoreQueue {
    semaphore: Semaphore,
}

impl Default for SemaphoreQueue {
    /// Create a `SemaphoreQueue` with a default permit count of **5**.
    #[inline]
    fn default() -> Self {
        SemaphoreQueue {
            semaphore: Semaphore::new(5),
        }
    }
}

impl SemaphoreQueue {
    /// Create a `SemaphoreQueue` allowing at most `n` concurrent executions.
    #[inline]
    pub fn new(n: usize) -> SemaphoreQueue {
        SemaphoreQueue {
            semaphore: Semaphore::new(n),
        }
    }

    /// Acquire one semaphore permit and run `call(arg)` asynchronously.
    ///
    /// If the maximum concurrency has been reached the caller suspends until
    /// a permit becomes available.  The permit is released automatically when
    /// the future produced by `call` resolves.
    #[inline]
    pub async fn run<A, T, R>(&self, call: impl FnOnce(A) -> T, arg: A) -> R
    where
        T: Future<Output = R>,
    {
        let _guard = self.semaphore.acquire().await;
        call(arg).await
    }
}
