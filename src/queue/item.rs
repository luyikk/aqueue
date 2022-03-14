use crate::queue::IQueueItem;
use anyhow::{anyhow, Result};
use async_oneshot::{oneshot, Receiver, Sender};
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

pub type BoxFuture<'a, S> = Pin<Box<dyn Future<Output = Result<S>> + Send + 'a>>;

/// AQueue run item
pub struct QueueItem<'a, S> {
    call: BoxFuture<'a, S>,
    result_sender: Sender<Result<S>>,
}

impl<'a, S> Future for QueueItem<'a, S>
where
    S: Sync + Send + 'static,
{
    type Output = Result<()>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let my = Pin::into_inner(self);
        match Pin::new(&mut my.call).poll(cx) {
            Poll::Ready(r) => Poll::Ready(my.result_sender.send(r).map_err(|_| anyhow!("rx is close"))),
            Poll::Pending => Poll::Pending,
        }
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
