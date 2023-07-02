use std::cell::UnsafeCell;

// Please do not use it at will
pub struct InnerStore<T>(UnsafeCell<T>);
unsafe impl<T> Sync for InnerStore<T> {}
unsafe impl<T> Send for InnerStore<T> {}

impl<T> InnerStore<T> {
    #[inline]
    pub(crate) fn new(x: T) -> InnerStore<T> {
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
