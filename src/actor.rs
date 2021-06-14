use crate::AQueue;
use std::cell::UnsafeCell;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use anyhow::Result;

// Please do not use it at will
pub struct InnerStore<T>(UnsafeCell<T>);
unsafe impl<T> Sync for InnerStore<T> {}
unsafe impl<T> Send for InnerStore<T> {}

impl<T> InnerStore<T> {
    #[inline]
    fn new(x: T) -> InnerStore<T> {
        InnerStore(UnsafeCell::new(x))
    }
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }
}

pub struct Actor<I> {
    inner: Arc<InnerStore<I>>,
    queue: AQueue,
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

    #[inline]
    pub async fn inner_call<T, S>(&self, call: impl FnOnce(Arc<InnerStore<I>>) -> T ) -> Result<S>
    where
        T: Future<Output = Result<S>> + Send  + 'static,
        S: 'static+Sync+Send, {
        self.queue.run(call, self.inner.clone()).await
    }

    /// # Safety
    #[inline]
    pub unsafe fn deref_inner(&self) -> RefInner<'_, I> {
        RefInner { value: self.inner.get() }
    }

    /// # Safety
    ///
    /// 捕获闭包的借用参数，可能会导致问题，请勿乱用
    pub async unsafe fn inner_call_ref<'a,T,S>(&self, call: impl FnOnce(Arc<InnerStore<I>>) -> T ) -> Result<S>
        where
            T: Future<Output = Result<S>> + Send  + 'a,
            S: 'static+Sync+Send, {
        self.queue.ref_run(call, self.inner.clone()).await
    }
}
