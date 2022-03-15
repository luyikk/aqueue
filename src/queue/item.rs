use crate::queue::IQueueItem;
use anyhow::{anyhow, Result};
use async_oneshot::{oneshot, Receiver, Sender};
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use futures_util::ready;

pub type BoxFuture<'a, S> = Pin<Box<dyn Future<Output = Result<S>> + Send + 'a>>;

pin_project_lite::pin_project! {
    /// AQueue run item
    pub struct QueueItem<'a, S> {
        #[pin]
        call: BoxFuture<'a, S>,
        result_sender: Sender<Result<S>>,
    }
}

impl<'a, S> Future for QueueItem<'a, S>
where
    S: Sync + Send + 'static,
{
    type Output = Result<()>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let my = self.project();
        Poll::Ready( match my.result_sender.send(ready!(my.call.poll(cx))){
            Ok(_)=>Ok(()),
            Err(_)=>Err(anyhow!("rx is close"))
        })
    }
}

impl<'a, S> IQueueItem for QueueItem<'a, S> where S: Sync + Send + 'static {}

impl<'a, S> QueueItem<'a, S>
where
    S: Sync + Send + 'static,
{
    #[inline]
    pub fn new(call: BoxFuture<'a, S>) -> (Receiver<Result<S>>, Self) {
        let (result_sender, rx) = oneshot();
        (rx, Self { call, result_sender })
    }
}
