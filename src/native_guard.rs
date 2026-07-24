//! Serializes non-thread-safe NBIS native entry points.
//!
//! - **Extract lock**: mindtct (`get_minutiae`), SIVV/OpenCV, NFIQ2 — one at a time per process.
//! - **Match lock**: legacy fallback only. After Bozorth thread-local workspace (≥ 0.1.18),
//!   `Minutiae::compare` does **not** need this lock and can run in parallel.

use std::sync::{LazyLock, Mutex};

static NBIS_EXTRACT_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static NBIS_MATCH_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn lock_poison_ok(mutex: &'static Mutex<()>) -> std::sync::MutexGuard<'static, ()> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Hold while calling mindtct / SIVV / NFIQ2.
pub(crate) fn with_extract_lock<R>(f: impl FnOnce() -> R) -> R {
    let _guard = lock_poison_ok(&NBIS_EXTRACT_MUTEX);
    f()
}

/// Hold only if Bozorth is still process-global. Prefer unlocked compare after TLS Bozorth.
#[allow(dead_code)]
pub(crate) fn with_match_lock<R>(f: impl FnOnce() -> R) -> R {
    let _guard = lock_poison_ok(&NBIS_MATCH_MUTEX);
    f()
}

/// Backward-compatible alias: locks extract path (historical “native” call sites).
#[inline]
#[allow(dead_code)]
pub(crate) fn with_native_lock<R>(f: impl FnOnce() -> R) -> R {
    with_extract_lock(f)
}
