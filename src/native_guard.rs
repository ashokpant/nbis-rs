//! Serializes all non-thread-safe NBIS / NFIQ2 / SIVV native entry points in one process.
//!
//! Mindtct (`get_minutiae`), Bozorth, and NFIQ2 are not safe to run concurrently in the same
//! address space. Per-instance `Mutex<Nfiq2>` and a separate Bozorth mutex are not enough when
//! multiple `NbisExtractor` instances run on different threads.

use std::sync::{LazyLock, Mutex};

static NBIS_NATIVE_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) fn with_native_lock<R>(f: impl FnOnce() -> R) -> R {
    let _guard = NBIS_NATIVE_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    f()
}
