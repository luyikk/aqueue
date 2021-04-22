use super::QueueItem;
use async_trait::async_trait;
use async_oneshot::{oneshot, Receiver, Sender};
use std::cell::RefCell;
use std::future::Future;
use anyhow::*;
use std::pin::Pin;

pub struct AQueueItem<S> {
    call: RefCell<Option<Pin<Box<dyn Future<Output = Result<S>> + Send>>>>,
    result_sender: RefCell<Option<Sender<Result<S>>>>,
}

unsafe impl<S> Send for AQueueItem<S> {}
unsafe impl<S> Sync for AQueueItem<S> {}

#[async_trait]
impl<S> QueueItem for AQueueItem<S>
where
    S: 'static+Sync+Send
{
    #[inline]
    async fn run(&self) -> Result<()> {

        let mut sender = self.result_sender.take().ok_or_else(|| anyhow!("not call one_shot is none"))?;
        if sender.send( self.run().await).is_err() {
            bail!("CLOSE")
        } else {
            Ok(())
        }

    }
}

impl<S> AQueueItem<S>
where
    S: 'static+Sync+Send
{
    #[inline]
    pub fn new(call:Pin<Box<dyn Future<Output = Result<S>> + Send>>) -> (Receiver<Result<S>>, Box<dyn QueueItem + Send + Sync>) {
        let (tx, rx) = oneshot();
        (
            rx,
            Box::new(AQueueItem {
                call: RefCell::new(Some(call)),
                result_sender:RefCell::new( Some(tx))
            }),
        )
    }

    #[inline]
    async fn run(&self)-> Result<S> {
        let call = self.call.take().ok_or_else(|| anyhow!("not call fn is none"))?;
        call.await
    }
}
