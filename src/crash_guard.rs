//! Convert SIGSEGV / SIGBUS / SIGFPE from legacy C into a Rust `Err`.
//!
//! Only safe while the process-wide extract lock is held (single-threaded C).
//! After a caught crash the C heap may be inconsistent — recycle the process
//! when possible; this still prevents the whole API from dying silently.

#![cfg(unix)]

use std::cell::Cell;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use libc::{c_int, sigaction, sigemptyset, sigset_t, SA_NODEFER, SIGBUS, SIGFPE, SIGSEGV, SIG_DFL};

/// Distinguish which native subsystem faulted (for logs / error messages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrashContext {
    Mindtct,
    Nfiq2,
    Sivv,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaughtCrash {
    pub signal: c_int,
    pub context: CrashContext,
}

// libc crate does not always export setjmp; declare the portable C ABI ourselves.
#[cfg(target_os = "macos")]
type JmpBuf = [u32; 48]; // enough for arm64/x86_64 jmp_buf on Darwin
#[cfg(all(unix, not(target_os = "macos")))]
type JmpBuf = [u64; 32]; // enough for glibc jmp_buf / sigjmp_buf storage

unsafe extern "C" {
    fn setjmp(env: *mut JmpBuf) -> c_int;
    fn longjmp(env: *mut JmpBuf, val: c_int) -> !;
}

static GUARD_ACTIVE: AtomicBool = AtomicBool::new(false);
static LAST_SIGNAL: AtomicI32 = AtomicI32::new(0);

thread_local! {
    static JUMP_BUF: Cell<*mut JmpBuf> = const { Cell::new(ptr::null_mut()) };
    static CRASH_CTX: Cell<CrashContext> = const { Cell::new(CrashContext::Other) };
}

extern "C" fn native_crash_handler(sig: c_int) {
    if !GUARD_ACTIVE.load(Ordering::SeqCst) {
        unsafe {
            libc::signal(sig, SIG_DFL);
            libc::raise(sig);
        }
        return;
    }
    LAST_SIGNAL.store(sig, Ordering::SeqCst);
    let buf = JUMP_BUF.with(|c| c.get());
    if buf.is_null() {
        unsafe {
            libc::signal(sig, SIG_DFL);
            libc::raise(sig);
        }
        return;
    }
    unsafe {
        longjmp(buf, 1);
    }
}

fn install_handlers() -> [libc::sigaction; 3] {
    let mut old = [unsafe { std::mem::zeroed::<libc::sigaction>() }; 3];
    let signals = [SIGSEGV, SIGBUS, SIGFPE];
    for (i, &sig) in signals.iter().enumerate() {
        let mut act: libc::sigaction = unsafe { std::mem::zeroed() };
        act.sa_sigaction = native_crash_handler as *const () as usize;
        unsafe {
            sigemptyset(&mut act.sa_mask as *mut sigset_t);
        }
        act.sa_flags = SA_NODEFER;
        unsafe {
            sigaction(sig, &act, &mut old[i]);
        }
    }
    old
}

fn restore_handlers(old: &[libc::sigaction; 3]) {
    let signals = [SIGSEGV, SIGBUS, SIGFPE];
    for (i, &sig) in signals.iter().enumerate() {
        unsafe {
            sigaction(sig, &old[i], ptr::null_mut());
        }
    }
}

/// Run `f` under a SIGSEGV/SIGBUS/SIGFPE catcher.
///
/// # Safety contract
/// Caller must hold `EXTRACT_LOCK` (no concurrent guarded C on other threads).
/// On `Err(CaughtCrash)` the process may be heap-corrupted — prefer recycling the worker.
pub(crate) fn with_native_crash_guard<T, F>(context: CrashContext, f: F) -> Result<T, CaughtCrash>
where
    F: FnOnce() -> T,
{
    CRASH_CTX.with(|c| c.set(context));
    LAST_SIGNAL.store(0, Ordering::SeqCst);

    let mut jmp_buf_storage: JmpBuf = unsafe { std::mem::zeroed() };
    JUMP_BUF.with(|c| c.set(&mut jmp_buf_storage as *mut _));

    let old = install_handlers();
    GUARD_ACTIVE.store(true, Ordering::SeqCst);

    let result = unsafe {
        if setjmp(&mut jmp_buf_storage as *mut _) == 0 {
            let value = f();
            Ok(value)
        } else {
            Err(CaughtCrash {
                signal: LAST_SIGNAL.load(Ordering::SeqCst),
                context: CRASH_CTX.with(|c| c.get()),
            })
        }
    };

    GUARD_ACTIVE.store(false, Ordering::SeqCst);
    JUMP_BUF.with(|c| c.set(ptr::null_mut()));
    restore_handlers(&old);
    result
}

impl CaughtCrash {
    pub(crate) fn message(&self) -> String {
        let ctx = match self.context {
            CrashContext::Mindtct => "mindtct",
            CrashContext::Nfiq2 => "nfiq2",
            CrashContext::Sivv => "sivv",
            CrashContext::Other => "native",
        };
        format!(
            "native {ctx} crash (signal {}); recycle worker process if this repeats",
            self.signal
        )
    }
}
