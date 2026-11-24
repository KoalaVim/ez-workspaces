//! Small fixed worker pool (design D3): std threads pulling jobs from a
//! queue and sending each job's result over the application's `mpsc`
//! channel. Results carry generation ids chosen by the submitter, so stale
//! ones are dropped on the main thread.

use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

/// Threads in the pool.
const THREADS: usize = 4;

type Job<M> = Box<dyn FnOnce() -> Option<M> + Send>;

struct Shared<M> {
    queue: Mutex<State<M>>,
    ready: Condvar,
}

struct State<M> {
    jobs: VecDeque<Job<M>>,
    closed: bool,
}

/// Handle to the pool; clones share the same threads.
pub struct Pool<M> {
    shared: Arc<Shared<M>>,
    /// Closes the queue when the last handle is dropped.
    _closer: Arc<Closer<M>>,
}

struct Closer<M>(Arc<Shared<M>>);

impl<M> Drop for Closer<M> {
    fn drop(&mut self) {
        self.0.queue.lock().unwrap().closed = true;
        self.0.ready.notify_all();
    }
}

impl<M> Clone for Pool<M> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
            _closer: Arc::clone(&self._closer),
        }
    }
}

impl<M: Send + 'static> Pool<M> {
    /// Start the threads; job results are sent to `tx`.
    pub fn new(tx: Sender<M>) -> Self {
        let shared = Arc::new(Shared {
            queue: Mutex::new(State {
                jobs: VecDeque::new(),
                closed: false,
            }),
            ready: Condvar::new(),
        });
        for n in 0..THREADS {
            let shared = Arc::clone(&shared);
            let tx = tx.clone();
            thread::Builder::new()
                .name(format!("ez-worker-{n}"))
                .spawn(move || work(&shared, &tx))
                .expect("spawn worker thread");
        }
        let closer = Arc::new(Closer(Arc::clone(&shared)));
        Self {
            shared,
            _closer: closer,
        }
    }

    /// Queue a job behind the ones already waiting.
    pub fn submit(&self, job: impl FnOnce() -> Option<M> + Send + 'static) {
        self.push(Box::new(job), false);
    }

    /// Queue a job ahead of the waiting ones (row loads must not wait behind previews).
    pub fn submit_front(&self, job: impl FnOnce() -> Option<M> + Send + 'static) {
        self.push(Box::new(job), true);
    }

    fn push(&self, job: Job<M>, front: bool) {
        let mut state = self.shared.queue.lock().unwrap();
        if front {
            state.jobs.push_front(job);
        } else {
            state.jobs.push_back(job);
        }
        drop(state);
        self.shared.ready.notify_one();
    }
}

fn work<M>(shared: &Shared<M>, tx: &Sender<M>) {
    loop {
        let job = {
            let mut state = shared.queue.lock().unwrap();
            loop {
                if let Some(job) = state.jobs.pop_front() {
                    break job;
                }
                if state.closed {
                    return;
                }
                state = shared.ready.wait(state).unwrap();
            }
        };
        // A panicking job must not take the thread (and the pool) down.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
        if let Ok(Some(msg)) = outcome {
            // The receiver is gone when the browser has already exited.
            let _ = tx.send(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn results_arrive_over_the_channel() {
        let (tx, rx) = mpsc::channel();
        let pool = Pool::new(tx);
        for n in 0..10u32 {
            pool.submit(move || Some(n));
        }
        pool.submit(|| None);
        let mut got: Vec<u32> = (0..10)
            .map(|_| rx.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect();
        got.sort_unstable();
        assert_eq!(got, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn threads_stop_when_the_last_handle_is_dropped() {
        let (tx, rx) = mpsc::channel::<u32>();
        let pool = Pool::new(tx);
        let clone = pool.clone();
        drop(pool);
        clone.submit_front(|| Some(7));
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), 7);
        drop(clone);
        // All senders (one per thread) are gone once the threads exited.
        assert!(rx.recv_timeout(Duration::from_secs(5)).is_err());
    }
}
