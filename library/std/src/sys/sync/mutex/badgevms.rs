use crate::hint;
use crate::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use crate::sync::atomic::{Atomic, AtomicU8};
use crate::sys::pal::process as pal_process;

pub struct Mutex {
    state: Atomic<u8>,
}

const UNLOCKED: u8 = 0;
const LOCKED: u8 = 1;

impl Mutex {
    #[inline]
    pub const fn new() -> Mutex {
        Mutex { state: AtomicU8::new(UNLOCKED) }
    }

    #[inline]
    pub fn try_lock(&self) -> bool {
        self.state.compare_exchange(UNLOCKED, LOCKED, Acquire, Relaxed).is_ok()
    }

    #[inline]
    pub fn lock(&self) {
        let mut spins = 0;
        while !self.try_lock() {
            if spins < 64 {
                hint::spin_loop();
                spins += 1;
            } else {
                pal_process::yield_now();
                spins = 0;
            }
        }
    }

    #[inline]
    pub unsafe fn unlock(&self) {
        self.state.store(UNLOCKED, Release);
    }
}
