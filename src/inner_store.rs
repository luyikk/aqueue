use std::cell::UnsafeCell;

/// Internal value store backed by [`UnsafeCell`].
///
/// # Safety
///
/// `InnerStore<T>` opts in to `Send` and `Sync` unconditionally.
/// Thread safety is **not** enforced by the type system — it is the
/// responsibility of the surrounding queue primitive (mutex, rwlock, or
/// semaphore) to guarantee that at most one mutable reference exists at any
/// point in time.  Do **not** use this type directly outside of the queue
/// implementations in this crate.
pub struct InnerStore<T>(UnsafeCell<T>);
unsafe impl<T> Sync for InnerStore<T> {}
unsafe impl<T> Send for InnerStore<T> {}

impl<T> InnerStore<T> {
    /// Wrap `x` in an `InnerStore`.
    #[inline]
    pub(crate) fn new(x: T) -> InnerStore<T> {
        InnerStore(UnsafeCell::new(x))
    }

    /// Return a mutable reference to the inner value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other reference (mutable or shared) to
    /// the inner value exists concurrently.  This is guaranteed when called
    /// from within a properly acquired write lock.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }

    /// Return a shared reference to the inner value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no mutable reference to the inner value
    /// exists concurrently.  This is guaranteed when called from within a
    /// properly acquired read lock.
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }
}
