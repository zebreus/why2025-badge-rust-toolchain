use crate::ffi::c_void;
use crate::ptr;
use crate::sync::{Mutex, MutexGuard};
use crate::sys::pal::process as pal_process;

pub type Key = usize;
type Dtor = unsafe extern "C" fn(*mut u8);

struct ThreadTable {
    pid: pal_process::Pid,
    values: Vec<*mut u8>,
}

unsafe impl Send for ThreadTable {}

struct TlsRuntime {
    next_key: Key,
    dtors: Vec<(Key, Dtor)>,
    threads: Vec<ThreadTable>,
}

impl TlsRuntime {
    const fn new() -> Self {
        Self { next_key: 1, dtors: Vec::new(), threads: Vec::new() }
    }

    fn table_mut(&mut self, pid: pal_process::Pid) -> &mut ThreadTable {
        if let Some(index) = self.threads.iter().position(|table| table.pid == pid) {
            &mut self.threads[index]
        } else {
            self.threads.push(ThreadTable { pid, values: Vec::new() });
            self.threads.last_mut().unwrap()
        }
    }

    fn table(&self, pid: pal_process::Pid) -> Option<&ThreadTable> {
        self.threads.iter().find(|table| table.pid == pid)
    }
}

static TLS: Mutex<TlsRuntime> = Mutex::new(TlsRuntime::new());

#[inline]
pub fn create(dtor: Option<Dtor>) -> Key {
    let mut tls = tls_runtime();
    let key = tls.next_key;
    tls.next_key = tls.next_key.checked_add(1).unwrap_or_else(|| rtabort!("out of TLS keys"));
    if let Some(dtor) = dtor {
        tls.dtors.push((key, dtor));
    }
    key
}

#[inline]
pub unsafe fn set(key: Key, value: *mut u8) {
    let pid = pal_process::getpid();
    let mut tls = tls_runtime();
    let table = tls.table_mut(pid);
    if table.values.len() <= key {
        table.values.resize(key + 1, ptr::null_mut());
    }
    table.values[key] = value;
}

#[inline]
pub unsafe fn get(key: Key) -> *mut u8 {
    let pid = pal_process::getpid();
    tls_runtime()
        .table(pid)
        .and_then(|table| table.values.get(key).copied())
        .unwrap_or(ptr::null_mut())
}

#[inline]
pub unsafe fn destroy(key: Key) {
    let mut tls = tls_runtime();
    tls.dtors.retain(|(dtor_key, _)| *dtor_key != key);
    for table in &mut tls.threads {
        if let Some(value) = table.values.get_mut(key) {
            *value = ptr::null_mut();
        }
    }
}

pub unsafe fn destroy_current_thread_tls() {
    let pid = pal_process::getpid();

    for _ in 0..5 {
        let to_run = take_current_dtor_values(pid);
        if to_run.is_empty() {
            break;
        }

        for (_, ptr, dtor) in to_run {
            unsafe { dtor(ptr.cast::<c_void>().cast::<u8>()) };
        }
    }

    tls_runtime().threads.retain(|table| table.pid != pid);
    crate::rt::thread_cleanup();
}

fn take_current_dtor_values(pid: pal_process::Pid) -> Vec<(Key, *mut u8, Dtor)> {
    let mut tls = tls_runtime();
    let dtors = tls.dtors.clone();
    let Some(table) = tls.threads.iter_mut().find(|table| table.pid == pid) else {
        return Vec::new();
    };

    let mut to_run = Vec::new();
    for (key, dtor) in dtors {
        let Some(value) = table.values.get_mut(key) else {
            continue;
        };
        if value.is_null() {
            continue;
        }

        let ptr = *value;
        *value = ptr::null_mut();
        to_run.push((key, ptr, dtor));
    }
    to_run
}

fn tls_runtime() -> MutexGuard<'static, TlsRuntime> {
    TLS.lock().unwrap()
}
