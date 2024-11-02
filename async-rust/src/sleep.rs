use crate::runtime::schedule_wake_time;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

pub struct Sleep {
    wake_time: Instant,
}

pub fn sleep(duration: Duration) -> Sleep {
    Sleep {
        wake_time: Instant::now().checked_add(duration).unwrap(),
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("sleep polled");
        if self.wake_time < Instant::now() {
            Poll::Ready(())
        } else {
            // Schedule wake time insted of self-wake to avoid busy loop.
            // And to be waked again.
            schedule_wake_time(self.wake_time, cx.waker().clone());
            Poll::Pending
        }
    }
}
