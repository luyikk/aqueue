
pub mod queue;
pub use queue::{AQueueItem,AQueue,QueueItem};

pub use aqueue_trait::async_trait as aqueue_trait;
use std::cell::UnsafeCell;

pub struct InnerStore<T>(UnsafeCell<T>);
unsafe impl<T> Sync for InnerStore<T>{}
unsafe impl<T> Send for InnerStore<T>{}
impl<T> InnerStore<T>{
    pub fn new(x:T)-> InnerStore<T>{
        InnerStore(UnsafeCell::new(x))
    }
    pub fn get_mut(&self)->&mut T{
        unsafe{
            &mut *self.0.get()
        }
    }

    pub fn get(&self)->&T{
        unsafe{
            &*self.0.get()
        }
    }
}