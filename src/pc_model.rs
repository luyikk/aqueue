use super::semaphore::SemaphoreQueue;
use std::future::Future;

/// A thread-safe model that limits the number of concurrently executing
/// async operations using a semaphore.
///
/// `PCModel<I>` is useful for rate-limiting access to a resource — for
/// example capping the number of simultaneous HTTP requests or database
/// queries.  At most `n` closures (as configured in [`new`]) run at the same
/// time; additional callers suspend until a permit becomes available.
///
/// # Example
///
/// ```rust
/// use aqueue::PCModel;
/// use std::sync::Arc;
///
/// struct HttpClient;
/// impl HttpClient {
///     async fn get(&self, _url: &str) -> Vec<u8> { vec![] }
/// }
///
/// # #[tokio::main] async fn main() {
/// // Allow at most 4 concurrent requests
/// let client = Arc::new(PCModel::new(HttpClient, 4));
/// client.call(|c| async move { c.get("https://example.com").await }).await;
/// # }
/// ```
///
/// [`new`]: PCModel::new
pub struct PCModel<I> {
    inner: I,
    queue: SemaphoreQueue,
}

impl<I> PCModel<I> {
    /// Create a new `PCModel` wrapping `inner`, allowing at most `n`
    /// concurrent executions of [`call`].
    ///
    /// [`call`]: PCModel::call
    #[inline]
    pub fn new(inner: I, n: usize) -> Self {
        PCModel {
            inner,
            queue: SemaphoreQueue::new(n),
        }
    }

    /// Get a direct reference to the inner value, **bypassing the semaphore entirely**.
    ///
    /// # Warning
    ///
    /// This method does **not** acquire any semaphore permit. Any method calls made
    /// through the returned reference are **not** subject to the concurrency limit
    /// configured in [`PCModel::new`]. Using this method to invoke operations that
    /// should be rate-limited will silently defeat the purpose of `PCModel`.
    ///
    /// Only use this method to read metadata that is safe to access concurrently
    /// without counting against the parallelism budget (e.g. a connection-pool
    /// address, a configuration flag, etc.).
    ///
    /// To run code with the concurrency limit enforced, use [`PCModel::call`] instead.
    #[inline]
    pub fn inner(&self) -> &I {
        &self.inner
    }

    /// Acquire one semaphore permit and run `call` asynchronously.
    ///
    /// The closure receives a shared reference `&'a I` valid for the lifetime
    /// of the `PCModel`.  If `n` permits are already held, the caller suspends
    /// until one is released.
    #[inline]
    pub async fn call<'a, T, R>(&'a self, call: impl FnOnce(&'a I) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.run(call, &self.inner).await
    }
}
