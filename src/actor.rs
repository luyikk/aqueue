use crate::AQueue;

use crate::inner_store::InnerStore;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;

pub struct Actor<I> {
    inner: Arc<InnerStore<I>>,
    queue: AQueue,
}

impl<I: Default> Default for Actor<I> {
    fn default() -> Self {
        Self {
            inner: Arc::new(InnerStore::new(Default::default())),
            queue: AQueue::new(),
        }
    }
}

pub struct RefInner<'a, T: ?Sized> {
    value: &'a T,
}

impl<T: ?Sized> Deref for RefInner<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<I: 'static> Actor<I> {
    #[inline]
    pub fn new(x: I) -> Actor<I> {
        Actor {
            inner: Arc::new(InnerStore::new(x)),
            queue: AQueue::new(),
        }
    }

    /// Behavior through queues,thread safe call async fn
    #[inline]
    pub async fn inner_call<T, R>(&self, call: impl FnOnce(Arc<InnerStore<I>>) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.run(call, self.inner.clone()).await
    }

    /// # Safety
    /// This is a thread unsafe way to get
    /// When using, please make sure there is no thread safety problem
    #[inline]
    pub unsafe fn deref_inner(&self) -> RefInner<'_, I> {
        RefInner { value: self.inner.get() }
    }
}
