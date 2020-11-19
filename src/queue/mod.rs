mod fn_one;
use async_trait::async_trait;

#[async_trait]
pub trait QueueItem{
    async fn run(&self)->Result<(), Box<dyn Error+Send+Sync>>;
}

use std::error::Error;

pub use fn_one::AQueueItem;
use std::sync::atomic::{AtomicU8, Ordering};
use async_oneshot::{ Receiver};
use deque::{Worker,Stealer,new} ;
use std::future::Future;
use deque::Stolen::Data;

const IDLE:u8=0;
const OPEN:u8=1;


pub struct AQueue{
    worker:Worker<Box<dyn QueueItem+Send+Sync>>,
    stealer:Stealer<Box<dyn QueueItem+Send+Sync>>,
    status:AtomicU8
}

unsafe impl Send for AQueue{}
unsafe impl Sync for AQueue{}

impl AQueue{
    pub fn new()->AQueue{
        let (wk,stl)=new();
        AQueue{
            worker:wk,
            stealer:stl,
            status:AtomicU8::new(IDLE)
        }
    }

    #[inline]
    pub async fn run<A,T,S>(&self, call:impl FnOnce(A)->T+ Send+Sync+'static, arg:A) ->Result<S, Box<dyn Error+Send+Sync>>
    where T:Future<Output = Result<S, Box<dyn Error+Send+Sync>>> + Send+ Sync+'static,
          S:'static, A: Send+Sync+'static {
        let x= AQueueItem::new(call,arg);
        self.push(x).await
    }

    #[inline]
    pub async fn push<T>(&self,(rx,item):(Receiver<Result<T, Box<dyn Error+Send+Sync>>>,Box<dyn QueueItem+Send+Sync>))->Result<T, Box<dyn Error+Send+Sync>>{
        self.worker.push(item);
        self.run_ing().await?;
        match rx.await {
            Ok(x)=>Ok(x?),
            Err(_)=> Err("CLOSE".into())
        }
    }


    #[inline]
    pub async fn run_ing(&self)->Result<(), Box<dyn Error+Send+Sync>>{
        if  self.status.compare_and_swap(IDLE,OPEN,Ordering::AcqRel)==IDLE {
            loop {
                let item = {
                    match self.stealer.steal() {
                        Data(p)=>{
                            p
                        }
                        _ => {
                            if self.status.compare_and_swap(OPEN, IDLE, Ordering::AcqRel) == OPEN {
                                break;
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
