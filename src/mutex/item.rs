// use crate::mutex::IQueueItem;
// use anyhow::{anyhow, Result};
// use async_oneshot::{oneshot, Receiver, Sender};
// use futures_util::ready;
// use std::future::Future;
// use std::pin::Pin;
// use std::task::Poll;
//
// pin_project_lite::pin_project! {
//     /// AQueue run item
//     pub struct QueueItem<Fu:Future> {
//         #[pin]
//         call: Fu,
//         result_sender: Sender<Fu::Output>,
//     }
// }
//
// impl<Fu: Future> IQueueItem for QueueItem<Fu> {}
//
// impl<Fu> QueueItem<Fu>
// where
//     Fu: Future,
// {
//     #[inline]
//     pub fn new(call: Fu) -> (Receiver<Fu::Output>, Self) {
//         let (result_sender, result_receiver) = oneshot();
//         (result_receiver, Self { call, result_sender })
//     }
// }
//
// impl<Fu: Future> Future for QueueItem<Fu> {
//     type Output = Result<()>;
//
//     #[inline]
//     fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
//         let my = self.project();
//         Poll::Ready(my.result_sender.send(ready!(my.call.poll(cx))).map_err(|_| anyhow!("rx is close")))
//     }
// }
