use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::sleep::{sleep, Sleep};

pub struct Foo {
    n: u64,
    started: bool,
    timer: Pin<Box<Sleep>>,
}

pub fn foo(n: u64) -> Foo {
    // Usage of our custom async sleep implementation.
    let timer = sleep(Duration::from_secs(2));
    Foo {
        n,
        started: false,
        timer: Box::pin(timer),
    }
}

impl Future for Foo {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.started {
            println!("start: {}", self.n);
            self.started = true;
        }
        match self.timer.as_mut().poll(cx) {
            Poll::Ready(_) => {
                println!("end: {}", self.n);
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
