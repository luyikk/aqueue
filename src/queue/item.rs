use super::QueueItem;
use async_trait::async_trait;
use async_oneshot::{oneshot, Receiver, Sender};
use std::cell::RefCell;
use std::future::Future;
use anyhow::*;
use std::pin::Pin;

pub type FutureBox<'a,S>=Pin<Box<dyn Future<Output = Result<S>> + Send+'a>>;

pub struct AQueueItem<'a,S> {
    call: RefCell<Option<FutureBox<'a,S>>>,
    result_sender: RefCell<Option<Sender<Result<S>>>>,
}

unsafe impl<'a,S> Send for AQueueItem<'a,S> {}
unsafe impl<'a,S> Sync for AQueueItem<'a,S> {}

#[async_trait]
impl<'a,S> QueueItem for AQueueItem<'a,S>
where
    S: 'static+Sync+Send
{
    #[inline]
    async fn run(&self) -> Result<()> {
        let mut sender = self.result_sender.take().ok_or_else(|| anyhow!("not call one_shot is none"))?;
        sender.send( self.run().await).map_err(|_|anyhow!("rx is close"))
    }
}

impl<'a,S> AQueueItem<'a,S>
where
    S: 'static+Sync+Send
{
    #[inline]
    pub fn new(call:FutureBox<'a,S>) -> (Receiver<Result<S>>, Self) {
        let (tx, rx) = oneshot();
        (
            rx,
            AQueueItem {
                call: RefCell::new(Some(call)),
                result_sender:RefCell::new( Some(tx))
            },
        )
    }

    #[inline]
    async fn run(&self)-> Result<S> {
        let call = self.call.take().ok_or_else(|| anyhow!("not call fn is none"))?;
        call.await
    }
}
