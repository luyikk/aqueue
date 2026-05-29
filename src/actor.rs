use crate::AQueue;

use crate::inner_store::InnerStore;
use std::future::Future;
use std::ops::Deref;

/// A thread-safe actor that serialises all access to an inner value `I`
/// through an async mutex queue.
///
/// Every call to [`inner_call`] acquires the internal [`AQueue`] lock before
/// running the supplied closure, ensuring that only one operation executes at
/// a time regardless of how many tasks call concurrently.
///
/// # Choosing between `Actor` and [`RwModel`]
///
/// | | `Actor<I>` | `RwModel<I>` |
/// |---|---|---|
/// | Concurrency | serial (one at a time) | concurrent reads / exclusive writes |
/// | Best for | write-heavy or stateful workloads | read-heavy workloads |
///
/// [`inner_call`]: Actor::inner_call
/// [`RwModel`]: crate::RwModel
pub struct Actor<I> {
    inner: InnerStore<I>,
    queue: AQueue,
}

impl<I: Default> Default for Actor<I> {
    fn default() -> Self {
        Self {
            inner: InnerStore::new(Default::default()),
            queue: AQueue::new(),
        }
    }
}

/// A smart pointer that wraps a shared reference to `T` and implements
/// [`Deref`] for ergonomic access.  Returned by [`Actor::deref_inner`].
pub struct RefInner<'a, T: ?Sized> {
    pub(crate) value: &'a T,
}

impl<T: ?Sized> Deref for RefInner<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<I> Actor<I> {
    /// Create a new `Actor` wrapping the given value.
    #[inline]
    pub fn new(x: I) -> Actor<I> {
        Actor {
            inner: InnerStore::new(x),
            queue: AQueue::new(),
        }
    }

    /// Run an async closure under the exclusive queue lock.
    ///
    /// The closure receives a reference to the [`InnerStore`] whose lifetime
    /// `'a` is tied to `&'a self`, ensuring the reference remains valid for
    /// the entire duration of the async operation.
    ///
    /// All concurrent callers are queued and executed one at a time in
    /// arrival order (FIFO).
    ///
    /// # Example
    ///
    /// ```rust
    /// use aqueue::Actor;
    ///
    /// struct Counter { n: u64 }
    ///
    /// # #[tokio::main] async fn main() {
    /// let actor = Actor::new(Counter { n: 0 });
    /// actor.inner_call(|inner| async move {
    ///     inner.get_mut().n += 1;
    /// }).await;
    /// # }
    /// ```
    #[inline]
    pub async fn inner_call<'a, T, R>(&'a self, call: impl FnOnce(&'a InnerStore<I>) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.run(call, &self.inner).await
    }

    /// Obtain a direct reference to the inner value **without** acquiring the
    /// queue lock.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that no [`inner_call`] closure is executing
    /// concurrently.  Violating this guarantee causes undefined behaviour due
    /// to data races.
    ///
    /// This escape hatch is useful for reading plain data that is never
    /// mutated after construction, where queueing overhead is unacceptable.
    ///
    /// [`inner_call`]: Actor::inner_call
    #[inline]
    pub unsafe fn deref_inner(&self) -> RefInner<'_, I> {
        RefInner { value: self.inner.get() }
    }
}
