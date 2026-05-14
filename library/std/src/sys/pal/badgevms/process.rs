use why2025_badge_sys_bindings as abi;

use crate::ffi::{CStr, c_char, c_int};
use crate::sync::{Mutex, MutexGuard};
use crate::time::Duration;
use crate::{cmp, io, ptr};

pub(crate) type Pid = abi::pid_t;
pub(crate) type TaskId = u64;

const YIELD_SLEEP_MICROS: u64 = 0;

#[inline]
pub(crate) fn spawn(path: &CStr, argv: &mut [*mut c_char]) -> io::Result<Pid> {
    if argv.len() > c_int::MAX as usize {
        return Err(io::const_error!(
            io::ErrorKind::InvalidInput,
            "too many process arguments for BadgeVMS"
        ));
    }

    let argc = argv.len() as c_int;
    let argv = if argv.is_empty() { ptr::null_mut() } else { argv.as_mut_ptr() };
    let pid = unsafe { abi::process_create(path.as_ptr(), 0, argc, argv) };

    if pid < 0 { Err(io::Error::last_os_error()) } else { Ok(pid) }
}

#[inline]
pub(crate) fn spawn_thread(
    entry: unsafe extern "C" fn(user_data: *mut crate::ffi::c_void),
    user_data: *mut crate::ffi::c_void,
    stack_size: u16,
) -> io::Result<Pid> {
    let pid = unsafe { abi::thread_create(Some(entry), user_data, stack_size) };

    if pid < 0 { Err(io::Error::last_os_error()) } else { Ok(pid) }
}

#[inline]
pub(crate) fn wait(block: bool, timeout_msec: u32) -> io::Result<Option<Pid>> {
    let pid = unsafe { abi::wait(block, timeout_msec) };

    if pid < 0 {
        if block { Err(io::Error::last_os_error()) } else { Ok(None) }
    } else {
        Ok(Some(pid))
    }
}

#[inline]
pub(crate) fn getpid() -> Pid {
    unsafe { abi::getpid() }
}

#[inline]
pub(crate) fn yield_now() {
    sleep_micros(YIELD_SLEEP_MICROS);
}

#[inline]
pub(crate) fn sleep(duration: Duration) {
    let micros = duration.as_micros().min(u64::MAX as u128) as u64;
    sleep_micros(micros);
}

#[inline]
pub(crate) fn sleep_micros(micros: u64) {
    let mut remaining = micros;
    loop {
        let chunk = cmp::min(remaining, abi::useconds_t::MAX as u64) as abi::useconds_t;
        let _ = unsafe { abi::usleep(chunk) };
        if remaining <= chunk as u64 {
            break;
        }
        remaining -= chunk as u64;
    }
}

pub(crate) fn register_task(pid: Pid) -> TaskId {
    task_registry().register(pid)
}

pub(crate) fn unregister_task(id: TaskId) {
    task_registry().unregister(id);
}

pub(crate) fn wait_for_task(id: TaskId) -> io::Result<()> {
    loop {
        if task_registry().take_completed(id) {
            return Ok(());
        }

        let Some(pid) = wait(true, 0)? else {
            continue;
        };
        task_registry().record_observed_pid(pid);
    }
}

pub(crate) fn try_wait_for_task(id: TaskId) -> io::Result<bool> {
    if task_registry().take_completed(id) {
        return Ok(true);
    }

    let Some(pid) = wait(false, 0)? else {
        return Ok(false);
    };
    task_registry().record_observed_pid(pid);

    Ok(task_registry().take_completed(id))
}

struct TaskRegistry {
    next_id: TaskId,
    pending: Vec<(Pid, Vec<TaskId>)>,
    completed: Vec<TaskId>,
}

impl TaskRegistry {
    const fn new() -> Self {
        Self { next_id: 1, pending: Vec::new(), completed: Vec::new() }
    }

    fn register(&mut self, pid: Pid) -> TaskId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);

        if let Some((_, ids)) = self.pending.iter_mut().find(|(pending_pid, _)| *pending_pid == pid)
        {
            ids.push(id);
        } else {
            self.pending.push((pid, vec![id]));
        }

        id
    }

    fn unregister(&mut self, id: TaskId) {
        if let Some(position) = self.completed.iter().position(|completed_id| *completed_id == id) {
            self.completed.swap_remove(position);
            return;
        }

        for index in 0..self.pending.len() {
            if let Some(id_position) =
                self.pending[index].1.iter().position(|pending_id| *pending_id == id)
            {
                self.pending[index].1.swap_remove(id_position);
                if self.pending[index].1.is_empty() {
                    self.pending.swap_remove(index);
                }
                return;
            }
        }
    }

    fn record_observed_pid(&mut self, pid: Pid) {
        let Some(position) = self.pending.iter().position(|(pending_pid, _)| *pending_pid == pid)
        else {
            return;
        };

        let id = self.pending[position].1.remove(0);
        if self.pending[position].1.is_empty() {
            self.pending.remove(position);
        }
        self.completed.push(id);
    }

    fn take_completed(&mut self, id: TaskId) -> bool {
        if let Some(position) = self.completed.iter().position(|completed_id| *completed_id == id) {
            self.completed.swap_remove(position);
            true
        } else {
            false
        }
    }
}

static TASK_REGISTRY: Mutex<TaskRegistry> = Mutex::new(TaskRegistry::new());

fn task_registry() -> MutexGuard<'static, TaskRegistry> {
    TASK_REGISTRY.lock().unwrap()
}
