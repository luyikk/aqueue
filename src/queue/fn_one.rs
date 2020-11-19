use std::future::Future;
use std::error::Error;
use aqueue_trait::async_trait;
use std::cell::RefCell;
use tokio::sync::oneshot::{Receiver,Sender,channel};
use super::QueueItem;


pub struct AQueueItem<A,T,S>{
    arg:RefCell<Option<A>>,
    call:RefCell<Option<Box<dyn FnOnce(A)->T+ Send+Sync>>>,
    result_sender:RefCell<Option<Sender<Result<S, Box<dyn Error+Send+Sync>>>>>
}

unsafe impl<A,T,S> Send for AQueueItem<A,T,S>{}
unsafe impl<A,T,S> Sync for AQueueItem<A,T,S>{}



#[async_trait]
impl<A,T,S> QueueItem for AQueueItem<A,T,S>
    where T:Future<Output = Result<S, Box<dyn Error+Send+Sync>>> + Send+ Sync, A: Send+Sync{

    #[inline]
    async fn run(&self) -> Result<(), Box<dyn Error+Send+Sync>> {
        let call = self.call.replace(None);

        if let Some(call) = call {
            let arg= self.arg.replace(None).unwrap();
            let res=  (call)(arg).await;

            if let Some(x)= self.result_sender.replace(None){
                if x.send(res).is_err() {
                    Err("close".into())
                }
                else{
                    Ok(())
                }
            }
            else{
                Err("not call oneshot is none".into())
            }
        }
        else {
            Err("not call fn is none".into())
        }
    }

}

impl <A,T,S> AQueueItem<A,T,S>
    where T:Future<Output = Result<S, Box<dyn Error+Send+Sync>>> + Send+ Sync+'static,
          S:'static, A: Send+Sync+'static {
    #[inline]
    pub fn new(call:impl FnOnce(A)->T+ Send+Sync+'static,arg:A)->(Receiver<Result<S, Box<dyn Error+Send+Sync>>>,Box<dyn QueueItem+Send+Sync>){
        let (tx,rx)=channel();
        (rx, Box::new(AQueueItem{
            arg:RefCell::new(Some(arg)),
            call:RefCell::new(Some(Box::new(call))),
            result_sender:RefCell::new(Some(tx))
        }))
    }

}

