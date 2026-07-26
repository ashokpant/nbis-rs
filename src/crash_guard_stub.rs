//! Non-Unix stub — native crash catching is Unix-only.

#![cfg(not(unix))]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrashContext {
    Mindtct,
    Nfiq2,
    Sivv,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaughtCrash {
    pub signal: i32,
    pub context: CrashContext,
}

impl CaughtCrash {
    pub(crate) fn message(&self) -> String {
        "native crash catching unavailable on this platform".into()
    }
}

pub(crate) fn with_native_crash_guard<T, F>(
    _context: CrashContext,
    f: F,
) -> Result<T, CaughtCrash>
where
    F: FnOnce() -> T,
{
    Ok(f())
}
