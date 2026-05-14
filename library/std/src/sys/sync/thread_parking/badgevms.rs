#![forbid(unsafe_op_in_unsafe_fn)]

use crate::pin::Pin;
use crate::sync::atomic::Ordering::{Acquire, Release};
use crate::sync::atomic::{Atomic, AtomicU8};
use crate::sys::pal::process as pal_process;
use crate::time::Duration;

const EMPTY: u8 = 0;
const PARKED: u8 = 1;
const NOTIFIED: u8 = 2;

pub struct Parker {
    state: Atomic<u8>,
}

impl Parker {
    pub unsafe fn new_in_place(parker: *mut Parker) {
        unsafe { parker.write(Parker { state: AtomicU8::new(EMPTY) }) };
    }

    pub unsafe fn park(self: Pin<&Self>) {
        if self.consume_token() {
            return;
        }

        let _ = self.state.compare_exchange(EMPTY, PARKED, Release, Acquire);
        loop {
            if self.consume_token() {
                return;
            }
            pal_process::sleep_micros(1_000);
        }
    }

    pub unsafe fn park_timeout(self: Pin<&Self>, dur: Duration) {
        if self.consume_token() {
            return;
        }

        let _ = self.state.compare_exchange(EMPTY, PARKED, Release, Acquire);
        let mut remaining = dur.as_micros().min(u64::MAX as u128) as u64;
        while remaining > 0 {
            if self.consume_token() {
                return;
            }
            let chunk = remaining.min(1_000);
            pal_process::sleep_micros(chunk);
            remaining = remaining.saturating_sub(chunk);
        }

        let _ = self.state.compare_exchange(PARKED, EMPTY, Acquire, Acquire);
    }

    #[inline]
    pub fn unpark(self: Pin<&Self>) {
        self.state.store(NOTIFIED, Release);
    }

    #[inline]
    fn consume_token(&self) -> bool {
        self.state.compare_exchange(NOTIFIED, EMPTY, Acquire, Acquire).is_ok()
    }
}
