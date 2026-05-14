use crate::cell::UnsafeCell;
use crate::ffi::{CStr, c_void};
use crate::io;
use crate::num::NonZero;
use crate::sync::Arc;
use crate::sys::pal::process as pal_process;
use crate::thread::ThreadInit;
use crate::time::Duration;

const MIN_STACK_SIZE: usize = 16 * 1024;
const MAX_STACK_SIZE: usize = u16::MAX as usize;

pub const DEFAULT_MIN_STACK_SIZE: usize = 0;

pub struct Thread {
    id: pal_process::TaskId,
    _inner: Arc<ThreadInner>,
}

struct ThreadInner {
    init: UnsafeCell<Option<Box<ThreadInit>>>,
}

unsafe impl Sync for ThreadInner {}
unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    /// # Safety
    ///
    /// See `thread::Builder::spawn_unchecked` for safety requirements.
    pub unsafe fn new(stack: usize, init: Box<ThreadInit>) -> io::Result<Thread> {
        let stack_size = normalize_stack_size(stack)?;
        let inner = Arc::new(ThreadInner { init: UnsafeCell::new(Some(init)) });
        let child_inner = Arc::clone(&inner);
        let user_data = Arc::into_raw(child_inner).cast_mut().cast::<c_void>();

        match pal_process::spawn_thread(trampoline, user_data, stack_size) {
            Ok(pid) => {
                let id = pal_process::register_task(pid);
                Ok(Thread { id, _inner: inner })
            }
            Err(error) => {
                drop(unsafe { Arc::from_raw(user_data.cast::<ThreadInner>()) });
                Err(error)
            }
        }
    }

    pub fn join(self) {
        let id = self.id;
        pal_process::wait_for_task(id)
            .unwrap_or_else(|_| rtabort!("failed to join BadgeVMS thread"));
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        pal_process::unregister_task(self.id);
    }
}

unsafe extern "C" fn trampoline(user_data: *mut c_void) {
    let inner = unsafe { Arc::from_raw(user_data.cast::<ThreadInner>()) };
    let init = unsafe { (&mut *inner.init.get()).take() }
        .unwrap_or_else(|| rtabort!("BadgeVMS thread init missing"));
    let rust_start = init.init();
    rust_start();
    unsafe { crate::sys::thread_local::key::destroy_current_thread_tls() };
}

fn normalize_stack_size(stack: usize) -> io::Result<u16> {
    if stack == 0 {
        return Ok(0);
    }

    let stack = stack.max(MIN_STACK_SIZE);
    if stack > MAX_STACK_SIZE {
        Err(io::const_error!(
            io::ErrorKind::InvalidInput,
            "BadgeVMS thread stack size exceeds u16 range"
        ))
    } else {
        Ok(stack as u16)
    }
}

pub fn available_parallelism() -> io::Result<NonZero<usize>> {
    Ok(NonZero::<usize>::MIN)
}

pub fn current_os_id() -> Option<u64> {
    Some(pal_process::getpid() as u64)
}

pub fn yield_now() {
    pal_process::yield_now();
}

pub fn set_name(_name: &CStr) {}

pub fn sleep(dur: Duration) {
    pal_process::sleep(dur);
}
