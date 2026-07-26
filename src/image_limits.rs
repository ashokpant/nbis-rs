//! Image size gates before calling legacy mindtct / NFIQ2 / SIVV.

use crate::NbisError;

/// Minimum width/height accepted by extract (mindtct is unreliable below this).
pub(crate) const MIN_EXTRACT_DIM: u32 = 32;
/// Soft minimum for NFIQ2 — smaller images skip quality and return score 0.
pub(crate) const MIN_NFIQ2_DIM: u32 = 96;
/// Upper bound to avoid huge allocations / pathological mindtct inputs.
pub(crate) const MAX_EXTRACT_DIM: u32 = 4096;

pub(crate) fn validate_extract_dims(width: u32, height: u32, buf_len: usize) -> Result<(), NbisError> {
    if width < MIN_EXTRACT_DIM || height < MIN_EXTRACT_DIM {
        return Err(NbisError::GenericError(format!(
            "image too small for NBIS extract: {width}x{height} (min {MIN_EXTRACT_DIM}x{MIN_EXTRACT_DIM})"
        )));
    }
    if width > MAX_EXTRACT_DIM || height > MAX_EXTRACT_DIM {
        return Err(NbisError::GenericError(format!(
            "image too large for NBIS extract: {width}x{height} (max {MAX_EXTRACT_DIM}x{MAX_EXTRACT_DIM})"
        )));
    }
    let expected = (width as usize).saturating_mul(height as usize);
    if buf_len != expected {
        return Err(NbisError::GenericError(format!(
            "grayscale buffer length {buf_len} != {width}*{height}={expected}"
        )));
    }
    Ok(())
}

pub(crate) fn nfiq2_image_ok(width: u32, height: u32) -> bool {
    width >= MIN_NFIQ2_DIM && height >= MIN_NFIQ2_DIM
}
