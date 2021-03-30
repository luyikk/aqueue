use super::QueueItem;
use crate::AResult;
use aqueue_trait::async_trait;
use async_oneshot::{oneshot, Receiver, Sender};
use std::cell::RefCell;
use std::future::Future;

pub struct AQueueItem<A, T, S> {
    arg: RefCell<Option<A>>,
    call: RefCell<Option<Box<dyn FnOnce(A) -> T + Send + Sync>>>,
    result_sender: RefCell<Option<Sender<AResult<S>>>>,
}

unsafe impl<A, T, S> Send for AQueueItem<A, T, S> {}
unsafe impl<A, T, S> Sync for AQueueItem<A, T, S> {}

#[async_trait]
impl<A, T, S> QueueItem for AQueueItem<A, T, S>
where
    T: Future<Output = AResult<S>> + Send + Sync,
    A: Send + Sync,
{
    #[inline]
    async fn run(&self) -> AResult<()> {
        let call = self.call.take();
        if let Some(call) =  call {
            let arg = self.arg.take().expect("arg is null");
            let res = (call)(arg).await;
            if let Some(mut x) = self.result_sender.take(){
                if x.send(res).is_err() {
                    Err("close".into())
                } else {
                    Ok(())
                }
            } else {
                Err("not call oneshot is none".into())
            }
        } else {
            Err("not call fn is none".into())
        }
    }
}

impl<A, T, S> AQueueItem<A, T, S>
where
    T: Future<Output = AResult<S>> + Send + Sync + 'static,
    S: 'static,
    A: Send + Sync + 'static,
{
    #[inline]
    pub fn new(call: impl FnOnce(A) -> T + Send + Sync + 'static, arg: A) -> (Receiver<AResult<S>>, Box<dyn QueueItem + Send + Sync>) {
        let (tx, rx) = oneshot();
        (
            rx,
            Box::new(AQueueItem {
                arg: RefCell::new(Some(arg)),
                call: RefCell::new(Some(Box::new(call))),
                result_sender: RefCell::new(Some(tx)),
            }),
        )
    }
}
