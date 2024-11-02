use crate::runtime::register_new_task;
use std::{
    future::Future,
    mem,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

pub fn spawn<F>(task: F) -> JoinHandle<F::Output>
where
    F: Future + Send + Sync + 'static,
    <F as futures::Future>::Output: std::marker::Send,
{
    let state = Arc::new(Mutex::new(JoinState::Unawaited));
    let task = Box::pin(wrap_with_join_state(task, Arc::clone(&state)));
    register_new_task(task);

    JoinHandle { state }
}

enum JoinState<T> {
    Unawaited,
    Awaited(Waker),
    Ready(T),
    Done,
}

pub struct JoinHandle<T> {
    state: Arc<Mutex<JoinState<T>>>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        match mem::replace(&mut *state, JoinState::Done) {
            JoinState::Awaited(_) | JoinState::Unawaited => {
                *state = JoinState::Awaited(cx.waker().clone());
                Poll::Pending
            }
            JoinState::Ready(value) => Poll::Ready(value),
            JoinState::Done => unreachable!(),
        }
    }
}

async fn wrap_with_join_state<F: Future>(future: F, join_state: Arc<Mutex<JoinState<F::Output>>>) {
    let value = future.await;
    let mut state = join_state.lock().unwrap();
    if let JoinState::Awaited(waker) = &*state {
        waker.wake_by_ref();
    }
    *state = JoinState::Ready(value);
}
