use crate::AQueue;
use anyhow::Result;
use std::cell::UnsafeCell;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;

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
    pub async fn inner_call<T, S>(&self, call: impl FnOnce(Arc<InnerStore<I>>) -> T) -> Result<S>
    where
        T: Future<Output = Result<S>> + Send + 'static,
        S: 'static + Sync + Send,
    {
        self.queue.run(call, self.inner.clone()).await
    }

    /// # Safety
    /// This is a thread unsafe way to get
    /// When using, please make sure there is no thread safety problem
    /// 因为获取的时候是直接抓取当前状态,并不是等待线程同步完成后拿取结果,所以请在需要的场合使用
    #[inline]
    pub unsafe fn deref_inner(&self) -> RefInner<'_, I> {
        RefInner { value: self.inner.get() }
    }

    /// # Safety
    /// self ref error!!
    /// Don't ref your &self
    /// 捕获闭包的借用参数，可能会导致自引用问题，请根据实际情况使用
    /// ``` ignore
    /// ///错误的示例; error example
    /// async fn error_example_func(&self, id: i32, desc: &str) -> Result<bool> {
    ///  unsafe {
    ///     self.inner_call_ref(async move |inner| {
    ///             // use &self error
    ///             let context= self.get_context().await?;
    ///             unimplemented!();
    ///         }).await
    ///     }
    /// }
    /// ```
    #[inline]
    pub async unsafe fn inner_call_ref<'a, T, S>(&'a self, call: impl FnOnce(Arc<InnerStore<I>>) -> T) -> Result<S>
    where
        T: Future<Output = Result<S>> + Send + 'a,
        S: 'static + Sync + Send,
    {
        self.queue.ref_run(call, self.inner.clone()).await
    }
}
