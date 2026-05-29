use crate::actor::RefInner;
use crate::inner_store::InnerStore;
use crate::RwQueue;
use std::future::Future;
use std::ops::{Deref, DerefMut};

/// A mutable smart pointer returned by [`RwModel::call_mut`].
///
/// Implements both [`Deref`] and [`DerefMut`] so callers can use it
/// transparently as `&mut T`.
pub struct RefMutInner<'a, T: ?Sized> {
    pub(crate) value: &'a mut T,
}

impl<'a, T> RefMutInner<'a, T> {
    /// Wrap a mutable reference.
    #[inline]
    pub fn new(value: &'a mut T) -> Self {
        Self { value }
    }
}

impl<T: ?Sized> Deref for RefMutInner<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T: ?Sized> DerefMut for RefMutInner<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

/// A thread-safe model that allows concurrent reads and exclusive writes,
/// backed by an async reader-writer lock queue.
///
/// `RwModel<I>` is ideal for **read-heavy** workloads where multiple tasks
/// read the inner value simultaneously, with occasional exclusive mutations.
///
/// # Concurrency semantics
///
/// - [`call`] acquires a **shared read lock** — multiple callers may run
///   concurrently.
/// - [`call_mut`] acquires an **exclusive write lock** — no other read or
///   write runs concurrently.
///
/// # Choosing between `RwModel` and [`Actor`]
///
/// | | `RwModel<I>` | `Actor<I>` |
/// |---|---|---|
/// | Reads | concurrent | serial |
/// | Writes | exclusive | serial |
/// | Best for | read-heavy | write-heavy / stateful |
///
/// [`call`]: RwModel::call
/// [`call_mut`]: RwModel::call_mut
/// [`Actor`]: crate::Actor
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

impl<I> RwModel<I> {
    /// Create a new `RwModel` wrapping the given value.
    #[inline]
    pub fn new(x: I) -> RwModel<I> {
        RwModel {
            inner: InnerStore::new(x),
            queue: RwQueue::new(),
        }
    }

    /// Acquire an exclusive write lock and run `call` asynchronously.
    ///
    /// The closure receives a [`RefMutInner<'a, I>`] whose lifetime is tied
    /// to `&'a self`, guaranteeing the mutable reference remains valid
    /// throughout the async operation.
    ///
    /// No other read or write operations run concurrently while the write
    /// lock is held.
    #[inline]
    pub async fn call_mut<'a, T, R>(&'a self, call: impl FnOnce(RefMutInner<'a, I>) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.write_run(call, self.inner.get_mut()).await
    }

    /// Acquire a shared read lock and run `call` asynchronously.
    ///
    /// The closure receives a [`RefInner<'a, I>`] whose lifetime is tied to
    /// `&'a self`.  Multiple read closures may execute concurrently.
    #[inline]
    pub async fn call<'a, T, R>(&'a self, call: impl FnOnce(RefInner<'a, I>) -> T) -> R
    where
        T: Future<Output = R>,
    {
        self.queue.read_run(call, self.inner.get()).await
    }

    /// Run `call` synchronously under a shared read lock.
    ///
    /// Yields the thread until the lock is available.  Prefer [`call`] from
    /// async contexts.
    ///
    /// [`call`]: RwModel::call
    #[inline]
    pub fn sync_call<R>(&self, call: impl FnOnce(&I) -> R) -> R {
        self.queue.sync_read_run(call, self.inner.get())
    }

    /// Run `call` synchronously under an exclusive write lock.
    ///
    /// Yields the thread until the lock is available.  Prefer [`call_mut`]
    /// from async contexts.
    ///
    /// [`call_mut`]: RwModel::call_mut
    #[inline]
    pub fn sync_mut_call<R>(&self, call: impl FnOnce(RefMutInner<'_, I>) -> R) -> R {
        self.queue.sync_write_run(call, RefMutInner { value: self.inner.get_mut() })
    }
}
