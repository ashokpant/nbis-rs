use std::sync::Arc;

use crate::minutia::Minutia;
use crate::structs::ROI;
use crate::Nfiq2Result;
use crate::{bozorth::bz_match_score, encoding::to_nist_xyt_set};
/// A set of minutiae extracted from a fingerprint image.
#[derive(Debug, Clone, uniffi::Object)]
pub struct Minutiae {
    pub(crate) inner: Vec<Minutia>,
    pub(crate) img_w: u32, // original image width for XYT conversion
    pub(crate) img_h: u32, // original image height for XYT conversion
    pub(crate) nfiq: Nfiq2Result,
    pub(crate) roi: Option<ROI>,
}

impl Minutiae {
    pub(crate) fn new(
        minutiae: Vec<Minutia>,
        img_w: u32,
        img_h: u32,
        nfiq: Nfiq2Result,
        roi: Option<ROI>,
    ) -> Self {
        Minutiae {
            inner: minutiae.to_vec(),
            img_w,
            img_h,
            nfiq,
            roi,
        }
    }
}

#[uniffi::export]
impl Minutiae {
    /// Similarity via Bozorth‑3 (higher = more similar). A score > 50 is a likely match.
    ///
    /// Thread-safe and parallel: Bozorth C workspace is thread-local (nbis-rs ≥ 0.1.18).
    ///
    /// # Arguments
    /// * `other` — another `Minutiae` object to compare against.
    ///
    /// Returns an `i32` score representing the similarity between the two sets of minutiae.
    /// A higher score indicates more similarity.
    pub fn compare(&self, other: &Minutiae) -> i32 {
        let p = to_nist_xyt_set(self);
        let g = to_nist_xyt_set(other);
        let score = bz_match_score(&p, &g);
        // #define QQ_SIZE 4000
        // #define QQ_OVERFLOW_SCORE QQ_SIZE
        // If the score is QQ_SIZE, it indicates an overflow condition.
        if score == 4000 {
            0 // Just return 0 for overflow
        } else {
            let score_opposite = bz_match_score(&g, &p);
            if score_opposite == 4000 {
                0 // Return 0 for overflow in the opposite direction
            } else if score > score_opposite {
                score
            } else {
                score_opposite
            }
        }
    }

    pub fn quality(&self) -> Nfiq2Result {
        self.nfiq.clone()
    }

    /// Returns a vector of `Minutia` objects representing the minutiae in this set.
    pub fn get(&self) -> Vec<Arc<Minutia>> {
        self.inner.iter().cloned().map(Arc::new).collect()
    }

    pub fn to_iso_19794_2_2011(&self) -> Result<Vec<u8>, crate::NbisError> {
        crate::encoding::to_iso_19794_2_2011(self)
    }

    /// Returns the ROI (Region of Interest) associated with these minutiae, if any.
    pub fn roi(&self) -> Option<ROI> {
        self.roi.clone()
    }
}
