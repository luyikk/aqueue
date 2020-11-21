use std::cell::UnsafeCell;
use crate::AQueue;
use std::future::Future;
use std::error::Error;
use std::sync::Arc;



// Please do not use it at will
pub struct InnerStore<T>(UnsafeCell<T>);
unsafe impl<T> Sync for InnerStore<T>{}
unsafe impl<T> Send for InnerStore<T>{}

impl<T> InnerStore<T>{
    #[inline]
    fn new(x:T)-> InnerStore<T>{
        InnerStore(UnsafeCell::new(x))
    }
    #[inline]
    pub fn get_mut(&self)->&mut T{
        unsafe {
            &mut *self.0.get()
        }
    }
    #[inline]
    pub fn get(&self)->&T {
        unsafe {
            &*self.0.get()
        }
    }
}




pub struct Actor<I>{
    inner:Arc<InnerStore<I>>,
    queue:AQueue
}


impl<I:'static> Actor<I>{

    #[inline]
    pub fn new(x:I)->Actor<I>{
        Actor{
            inner:Arc::new(InnerStore::new(x)),
            queue:AQueue::new()
        }
    }

    #[inline]
    pub async fn inner_call<T,S>(&self, call:impl FnOnce(Arc<InnerStore<I>>)->T+ Send+Sync+'static) ->Result<S, Box<dyn Error+Send+Sync>>
        where T:Future<Output = Result<S, Box<dyn Error+Send+Sync>>> + Send+ Sync+'static,
              S:'static {
        unsafe {
            self.queue.run(call, self.inner.clone()).await
        }
    }
}

