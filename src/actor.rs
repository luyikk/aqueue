use std::cell::UnsafeCell;
use crate::{AQueue, AResult};
use std::future::Future;
use std::sync::Arc;
use std::ops::Deref;


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

pub struct RefInner<'a,T:?Sized>{
    value:&'a T
}

impl<T: ?Sized> Deref for RefInner<'_,T>{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}


impl<I:'static> Actor<I>{

    #[inline]
    pub  fn new(x:I)->Actor<I>{
        Actor{
            inner:Arc::new(InnerStore::new(x)),
            queue:AQueue::new()
        }
    }

    #[inline]
    pub async fn inner_call<T,S>(&self, call:impl FnOnce(Arc<InnerStore<I>>)->T+ Send+Sync+'static) ->AResult<S>
        where T:Future<Output = AResult<S>> + Send+ Sync+'static,
              S:'static {
        unsafe {
            self.queue.run(call, self.inner.clone()).await
        }
    }

    #[inline]
    pub fn deref_inner(&self)->RefInner<I>{
        RefInner{
            value:self.inner.get()
        }
    }

}

