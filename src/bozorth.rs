use std::cmp::min;

use crate::ffi_nbis::{self};

use ffi_nbis::{
    xyt_struct,           // Bozorth input layout
    MAX_BOZORTH_MINUTIAE, // ‑‑”‑‑
};

/// Idiomatic Rust container for a minutiae set
#[derive(Clone, Debug)]
pub(crate) struct MinutiaeSet {
    pub xs: Vec<i32>,
    pub ys: Vec<i32>,
    pub theta: Vec<i32>,
}

impl MinutiaeSet {
    /// Converts to the C layout, truncating at MAX_BOZORTH_MINUTIAE
    fn to_c_struct(&self) -> xyt_struct {
        let mut xs = [0; MAX_BOZORTH_MINUTIAE];
        let mut ys = [0; MAX_BOZORTH_MINUTIAE];
        let mut theta = [0; MAX_BOZORTH_MINUTIAE];

        let n = min(self.xs.len(), MAX_BOZORTH_MINUTIAE);
        xs[..n].copy_from_slice(&self.xs[..n]);
        ys[..n].copy_from_slice(&self.ys[..n]);
        theta[..n].copy_from_slice(&self.theta[..n]);

        xyt_struct {
            nrows: n as i32,
            xcol: xs,
            ycol: ys,
            theta,
        }
    }
}

/// Safe wrapper around the C implementation.
pub(crate) fn bz_match_score(probe: &MinutiaeSet, gallery: &MinutiaeSet) -> i32 {
    let p_c = probe.to_c_struct();
    let g_c = gallery.to_c_struct();
    unsafe { ffi_nbis::bozorth_main(&p_c, &g_c) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dense_set(seed: i32, n: usize) -> MinutiaeSet {
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        let mut theta = Vec::with_capacity(n);
        for i in 0..n {
            let k = seed + i as i32;
            xs.push(20 + (k * 17) % 400);
            ys.push(20 + (k * 29) % 400);
            theta.push((k * 13) % 180);
        }
        MinutiaeSet { xs, ys, theta }
    }

    /// Dense 1:N compares used to segfault when qq[] overflow logged via fprintf(NULL).
    #[test]
    fn dense_gallery_compare_does_not_abort() {
        let probe = dense_set(1, 180);
        let mut saw_nonzero = false;
        for g in 0..40 {
            let gallery = dense_set(1000 + g * 7, 180);
            let score = bz_match_score(&probe, &gallery);
            assert!(score >= 0, "negative score {score} for gallery {g}");
            if score > 0 {
                saw_nonzero = true;
            }
        }
        // Overflow (4000) or any positive score means we exercised the matcher.
        assert!(saw_nonzero, "expected at least one non-zero Bozorth score");
    }
}
