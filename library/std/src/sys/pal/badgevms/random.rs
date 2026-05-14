use why2025_badge_sys_bindings as abi;

use crate::ffi::{c_int, c_uint, c_void};
use crate::ptr;
use crate::sync::Once;

const GETENTROPY_MAX: usize = 256;

static SEED_RANDOM_FALLBACK: Once = Once::new();

pub(crate) fn fill_bytes(bytes: &mut [u8]) {
    if getentropy(bytes).is_ok() {
        return;
    }

    fill_bytes_fallback(bytes);
}

fn getentropy(mut bytes: &mut [u8]) -> Result<(), ()> {
    while !bytes.is_empty() {
        let len = bytes.len().min(GETENTROPY_MAX);
        let ret = unsafe { abi::getentropy(bytes.as_mut_ptr().cast::<c_void>(), len) };
        if ret != 0 {
            return Err(());
        }
        bytes = &mut bytes[len..];
    }
    Ok(())
}

fn fill_bytes_fallback(bytes: &mut [u8]) {
    seed_random_fallback();

    let mut filled = 0;
    while filled < bytes.len() {
        let random = unsafe { abi::random() as u64 };
        for byte in random.to_ne_bytes() {
            if filled == bytes.len() {
                break;
            }
            bytes[filled] = byte;
            filled += 1;
        }
    }
}

fn seed_random_fallback() {
    SEED_RANDOM_FALLBACK.call_once(|| {
        let stack = 0u8;
        let mut seed = ptr::from_ref(&stack).addr() as u64;
        seed ^= crate::sys::pal::process::getpid() as u64;
        if let Ok(now) = crate::sys::pal::time::Timespec::wall_time() {
            seed ^= now.tv_sec as u64;
            seed ^= (now.tv_nsec.as_inner() as u64) << 32;
        }

        let seed = (seed ^ (seed >> 32)) as c_uint;
        unsafe {
            abi::srand(seed);
            abi::srandom(seed);
        }
    });
}

#[allow(dead_code)]
fn cvt(ret: c_int) -> Result<c_int, ()> {
    if ret < 0 { Err(()) } else { Ok(ret) }
}
