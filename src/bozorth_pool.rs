//! Thread pool sizing for parallel Bozorth (`NBIS_BOZORTH_THREADS`).

use std::sync::OnceLock;

/// Effective Bozorth worker count: env `NBIS_BOZORTH_THREADS`, else `min(available_parallelism, 8)`.
pub(crate) fn bozorth_thread_count() -> usize {
    static CACHED: OnceLock<usize> = OnceLock::new();
    *CACHED.get_or_init(|| {
        if let Ok(raw) = std::env::var("NBIS_BOZORTH_THREADS") {
            if let Ok(n) = raw.trim().parse::<usize>() {
                return n.max(1);
            }
        }
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        cpus.min(8).max(1)
    })
}
