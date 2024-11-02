use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::sleep::{sleep, Sleep};

pub struct Timeout<F> {
    sleep: Pin<Box<Sleep>>,
    inner: Pin<Box<F>>,
}

pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F> {
    Timeout {
        sleep: Box::pin(sleep(duration)),
        inner: Box::pin(future),
    }
}

impl<F: Future> Future for Timeout<F> {
    type Output = Option<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(output) = self.inner.as_mut().poll(cx) {
            println!("timeout completed");
            return Poll::Ready(Some(output));
        }
        if self.sleep.as_mut().poll(cx).is_ready() {
            println!("timeout aborted");
            return Poll::Ready(None);
        }
        Poll::Pending
    }
}
