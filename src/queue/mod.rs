mod fn_one;
use aqueue_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use async_oneshot::{ Receiver};
use std::future::Future;
use concurrent_queue::ConcurrentQueue;
pub use fn_one::AQueueItem;
use crate::AResult;

#[async_trait]
pub trait QueueItem{
    async fn run(&self)->AResult<()>;
}

const IDLE:u8=0;
const OPEN:u8=1;

pub struct AQueue{
    deque:ConcurrentQueue<Box<dyn QueueItem+Send+Sync>>,
    status:AtomicU8
}

unsafe impl Send for AQueue{}
unsafe impl Sync for AQueue{}

impl AQueue{
    pub fn new()->AQueue{
        AQueue{
            deque:ConcurrentQueue::unbounded(),
            status:AtomicU8::new(IDLE)
        }
    }

    #[inline]
    pub async fn run<A,T,S>(&self, call:impl FnOnce(A)->T+ Send+Sync+'static, arg:A) ->AResult<S>
    where T:Future<Output = AResult<S>> + Send+ Sync+'static,
          S:'static, A: Send+Sync+'static {
        self.push(AQueueItem::new(call,arg)).await
    }

    #[inline]
    pub async fn push<T>(&self,(rx,item):(Receiver<AResult<T>>,Box<dyn QueueItem+Send+Sync>))
        ->AResult<T>{
        if let Err(er)= self.deque.push(item){
            return Err(er.to_string().into())
        }
        self.run_ing().await?;
        match rx.await {
            Ok(x)=>Ok(x?),
            Err(_)=> Err("CLOSE".into())
        }
    }


    #[inline]
    pub async fn run_ing(&self)->AResult<()>{
        if  self.status.compare_exchange(IDLE,OPEN,Ordering::SeqCst,Ordering::Acquire)==Ok(IDLE) {
            'recv:loop {
                let item = {
                    match self.deque.pop() {
                        Ok(p)=>{
                            p
                        }
                        _ => {
                            if self.status.compare_exchange(OPEN, IDLE, Ordering::SeqCst,Ordering::Acquire) == Ok(OPEN) {
                                break 'recv;
                            } else {
                                panic!("error status")
                            }
                        }
                    }
                };

                item.run().await?;
            }

        }

        Ok(())
    }

}
