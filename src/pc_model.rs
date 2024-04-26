use super::semaphore::SemaphoreQueue;
use std::future::Future;

/// parallelism control model
/// The PCModel is a model that can be used to control task parallelism number
pub struct PCModel<I> {
    inner: I,
    queue: SemaphoreQueue,
}

impl<I> PCModel<I> {
    /// Create a new PCModel
    #[inline]
    pub fn new(inner: I, n: usize) -> Self {
        PCModel {
            inner,
            queue: SemaphoreQueue::new(n),
        }
    }

    /// Get the inner value reference
    #[inline]
    pub fn inner(&self) -> &I {
        &self.inner
    }

    /// Behavior through queues,thread parallelism control call async fn read ref
    #[inline]
    pub async fn call<'a, T, R>(&'a self, call: impl FnOnce(&'a I) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.run(call, &self.inner).await
    }
}
