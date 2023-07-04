use crate::inner_store::InnerStore;
use crate::RwQueue;
use std::future::Future;

/// RwModel
/// Ensure Thread safety and high performance reading and writing
pub struct RwModel<I> {
    inner: InnerStore<I>,
    queue: RwQueue,
}

impl<I: Default> Default for RwModel<I> {
    fn default() -> Self {
        Self {
            inner: InnerStore::new(Default::default()),
            queue: RwQueue::new(),
        }
    }
}

impl<I: 'static> RwModel<I> {
    #[inline]
    pub fn new(x: I) -> RwModel<I> {
        RwModel {
            inner: InnerStore::new(x),
            queue: RwQueue::new(),
        }
    }

    /// Behavior through queues,thread safe call async fn write ref mut
    #[inline]
    pub async fn call_mut<'a, T, R>(&'a self, call: impl FnOnce(&'a mut I) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.write_run(call, self.inner.get_mut()).await
    }

    /// Behavior through queues,thread safe call async fn read ref
    #[inline]
    pub async fn call<'a, T, R>(&'a self, call: impl FnOnce(&'a I) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.read_run(call, self.inner.get()).await
    }

    ///Thread safe call async fn read, Balanced queues are not supported
    #[inline]
    pub fn sync_call<R>(&self, call: impl FnOnce(&I) -> R) -> R {
        self.queue.sync_read_run(call, self.inner.get())
    }

    ///Thread safe call async fn write, Balanced queues are not supported
    #[inline]
    pub fn sync_mut_call<R>(&self, call: impl FnOnce(&mut I) -> R) -> R {
        self.queue.sync_read_run(call, self.inner.get_mut())
    }
}
