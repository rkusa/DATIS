use std::thread::{self, JoinHandle};

pub struct Worker<T> {
    join_handle: JoinHandle<T>,
}

impl<T> Worker<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        Worker {
            join_handle: thread::spawn(f),
        }
    }

    pub fn stop(self) {
        if let Err(_) = self.join_handle.join() {
            error!("Error joining worker thread");
        }
    }
}
