use crate::cell::UnsafeCell;
use crate::sys::sync::Mutex;
use crate::thread::{self, Thread};
use crate::time::Duration;

pub struct Condvar {
    waiters: UnsafeCell<Vec<Thread>>,
    queue_lock: Mutex,
}

unsafe impl Sync for Condvar {}

impl Condvar {
    #[inline]
    pub const fn new() -> Condvar {
        Condvar { waiters: UnsafeCell::new(Vec::new()), queue_lock: Mutex::new() }
    }

    pub fn notify_one(&self) {
        if let Some(thread) = self.pop_waiter() {
            thread.unpark();
        }
    }

    pub fn notify_all(&self) {
        for thread in self.take_waiters() {
            thread.unpark();
        }
    }

    pub unsafe fn wait(&self, mutex: &Mutex) {
        let thread = thread::current();
        self.push_waiter(thread.clone());
        unsafe { mutex.unlock() };
        unsafe { thread.park() };
        mutex.lock();
    }

    pub unsafe fn wait_timeout(&self, mutex: &Mutex, dur: Duration) -> bool {
        let thread = thread::current();
        let id = thread.id();
        self.push_waiter(thread.clone());
        unsafe { mutex.unlock() };

        unsafe { thread.park_timeout(dur) };
        let notified = !self.remove_waiter(id);
        mutex.lock();
        notified
    }

    fn push_waiter(&self, thread: Thread) {
        self.queue_lock.lock();
        unsafe { &mut *self.waiters.get() }.push(thread);
        unsafe { self.queue_lock.unlock() };
    }

    fn pop_waiter(&self) -> Option<Thread> {
        self.queue_lock.lock();
        let waiter = unsafe { &mut *self.waiters.get() }.pop();
        unsafe { self.queue_lock.unlock() };
        waiter
    }

    fn take_waiters(&self) -> Vec<Thread> {
        self.queue_lock.lock();
        let waiters = crate::mem::take(unsafe { &mut *self.waiters.get() });
        unsafe { self.queue_lock.unlock() };
        waiters
    }

    fn remove_waiter(&self, id: thread::ThreadId) -> bool {
        self.queue_lock.lock();
        let waiters = unsafe { &mut *self.waiters.get() };
        let removed = if let Some(position) = waiters.iter().position(|waiter| waiter.id() == id) {
            waiters.swap_remove(position);
            true
        } else {
            false
        };
        unsafe { self.queue_lock.unlock() };
        removed
    }
}
