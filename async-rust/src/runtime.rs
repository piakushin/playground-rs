use std::{
    collections::BTreeMap,
    io,
    os::fd::AsRawFd,
    sync::{Arc, Mutex},
    task::{Context, Wake, Waker},
    time::Instant,
};

use crate::DynFuture;

static NEW_TASKS: Mutex<Vec<DynFuture>> = Mutex::new(Vec::new());

static WAKE_TIME: Mutex<BTreeMap<Instant, Vec<Waker>>> = Mutex::new(BTreeMap::new());

static POLL_FDS: Mutex<Vec<libc::pollfd>> = Mutex::new(Vec::new());

static POLL_WAKERS: Mutex<Vec<Waker>> = Mutex::new(Vec::new());

pub fn schedule_wake_time(wake_time: Instant, waker: Waker) {
    let mut guard = WAKE_TIME.lock().unwrap();
    let wakers = guard.entry(wake_time).or_default();
    wakers.push(waker);
}

pub fn register_new_task(task: DynFuture) {
    NEW_TASKS.lock().unwrap().push(task);
}

pub fn register_poll_fd(context: &mut Context, fd: &impl AsRawFd, events: libc::c_short) {
    let mut poll_fds = POLL_FDS.lock().unwrap();
    let mut poll_wakers = POLL_WAKERS.lock().unwrap();
    poll_fds.push(libc::pollfd {
        fd: fd.as_raw_fd(),
        events,
        revents: 0,
    });
    poll_wakers.push(context.waker().clone());
}

struct AwakeFlag(Mutex<bool>);

impl AwakeFlag {
    fn check_and_clear(&self) -> bool {
        let mut guard = self.0.lock().unwrap();
        let check = *guard;
        *guard = false;
        check
    }
}

// When someone requests wake, flag is set and runtime doesn't pause until there is no wakes left.
impl Wake for AwakeFlag {
    fn wake(self: Arc<Self>) {
        *self.0.lock().unwrap() = true;
    }
}

pub fn enter(mut main_task: DynFuture) -> io::Result<()> {
    let awake_flag = Arc::new(AwakeFlag(Mutex::new(false)));
    let waker = Waker::from(Arc::clone(&awake_flag));
    let mut cx = Context::from_waker(&waker);
    let mut other_tasks: Vec<DynFuture> = Vec::new();
    loop {
        // If main task is completed, return and thus cancel all pending tasks.
        if main_task.as_mut().poll(&mut cx).is_ready() {
            return Ok(());
        }

        // Polling spawned tasks and removing completed.
        let is_pending = |task: &mut DynFuture| task.as_mut().poll(&mut cx).is_pending();
        other_tasks.retain_mut(is_pending);

        // Preparing spawned tasks to be polled on the next cycle.
        loop {
            let Some(mut new_task) = NEW_TASKS.lock().unwrap().pop() else {
                break;
            };
            if new_task.as_mut().poll(&mut cx).is_pending() {
                other_tasks.push(new_task);
            }
        }

        // If someone called wake, we can skip sleeping and continue to poll tasks.
        if awake_flag.check_and_clear() {
            continue;
        }

        // Asking OS to sleep until IO will be ready or wake up us when the earlier wake up is scheduled.
        let mut wake_times = WAKE_TIME.lock().unwrap();
        let timeout_ms = if let Some(next_wake) = wake_times.keys().next() {
            let duration = next_wake.saturating_duration_since(Instant::now());
            duration.as_millis() as libc::c_int
        } else {
            -1
        };

        let mut poll_fds = POLL_FDS.lock().unwrap();
        let mut poll_wakers = POLL_WAKERS.lock().unwrap();
        let poll_error_code = unsafe {
            libc::poll(
                poll_fds.as_mut_ptr(),
                poll_fds.len() as libc::nfds_t,
                timeout_ms,
            )
        };
        if poll_error_code < 0 {
            return Err(io::Error::last_os_error());
        }

        // If IO is not completed, we designed futures to reregister fds if io was not completed.
        poll_fds.clear();
        // Wake all futures waiting for IO.
        poll_wakers.drain(..).for_each(Waker::wake);

        // Wake all sleeping futures.
        while let Some(entry) = wake_times.first_entry() {
            if entry.key() > &Instant::now() {
                break;
            } else {
                entry.remove().into_iter().for_each(Waker::wake);
            }
        }
    }
}
