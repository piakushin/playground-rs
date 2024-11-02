use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct JoinAll<F> {
    futures: Vec<Pin<Box<F>>>,
}

pub fn join_all<F: Future>(futures: Vec<F>) -> JoinAll<F> {
    JoinAll {
        futures: futures.into_iter().map(|f| Box::pin(f)).collect(),
    }
}

impl<F: Future> Future for JoinAll<F> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let is_pending = |future: &mut Pin<Box<F>>| future.as_mut().poll(cx).is_pending();
        self.futures.retain_mut(is_pending);
        if self.futures.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
