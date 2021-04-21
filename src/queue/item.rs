use super::QueueItem;
use aqueue_trait::async_trait;
use async_oneshot::{oneshot, Receiver, Sender};
use std::cell::RefCell;
use std::future::Future;
use anyhow::*;

pub struct AQueueItem<A, T, S> {
    arg: RefCell<Option<A>>,
    call: RefCell<Option<Box<dyn FnOnce(A) -> T + Send + Sync>>>,
    result_sender: RefCell<Option<Sender<Result<S>>>>,
}

unsafe impl<A, T, S> Send for AQueueItem<A, T, S> {}
unsafe impl<A, T, S> Sync for AQueueItem<A, T, S> {}

#[async_trait]
impl<A, T, S> QueueItem for AQueueItem<A, T, S>
where
    T: Future<Output = Result<S>> + Send + Sync,
    A: Send + Sync,
{
    #[inline]
    async fn run(&self) -> Result<()> {
        let call = self.call.take().ok_or_else(|| anyhow!("not call fn is none"))?;
        let arg = self.arg.take().ok_or_else(|| anyhow!("arg is none"))?;
        let res = (call)(arg).await;
        let mut sender = self.result_sender.take().ok_or_else(|| anyhow!("not call one_shot is none"))?;
        if sender.send(res).is_err() {
            bail!("CLOSE")
        } else {
            Ok(())
        }
    }
}

impl<A, T, S> AQueueItem<A, T, S>
where
    T: Future<Output = Result<S>> + Send + Sync + 'static,
    S: 'static,
    A: Send + Sync + 'static,
{
    #[inline]
    pub fn new(call: impl FnOnce(A) -> T + Send + Sync + 'static, arg: A) -> (Receiver<Result<S>>, Box<dyn QueueItem + Send + Sync>) {
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
