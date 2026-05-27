use std::{
    ffi::CStr,
    os::raw::{c_char, c_uint, c_ushort},
    ptr,
};

use image::GrayImage;

use crate::{
    ffi_nfiq2::{
        nfiq2wrapper_compute, nfiq2wrapper_create, nfiq2wrapper_destroy, nfiq2wrapper_free_results,
        Nfiq2ResultsT, Nfiq2WrapperOpaque,
    },
    NbisError,
};

#[derive(Debug, Clone, uniffi::Record)]
pub struct Nfiq2Value {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Nfiq2Result {
    pub score: u32,
    pub actionable: Vec<Nfiq2Value>,
    pub features: Vec<Nfiq2Value>,
}

/// Handle to one NFIQ2 C++ model instance. Not `Clone` — each handle owns one `nfiq2wrapper_destroy`.
#[derive(Debug, uniffi::Object)]
pub struct Nfiq2 {
    ctx: *mut Nfiq2WrapperOpaque,
}

// Safety: `NbisExtractor` holds `Mutex<Nfiq2>` so `compute` never runs concurrently on one `ctx`.
// Do not share one `Nfiq2` across threads without external serialization.
unsafe impl Send for Nfiq2 {}
unsafe impl Sync for Nfiq2 {}

/// Construct a new wrapper, or Err if allocation/initialization fails.
pub fn new_nfiq2() -> Result<Nfiq2, NbisError> {
    let ptr = unsafe { nfiq2wrapper_create() };
    if ptr.is_null() {
        Err(NbisError::Nfiq2CreateFailed)
    } else {
        Ok(Nfiq2 { ctx: ptr })
    }
}

impl Nfiq2 {
    /// Compute quality from encoded image bytes (PNG/JPEG/…).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn compute(&self, image_bytes: &[u8]) -> Result<Nfiq2Result, NbisError> {
        let image =
            image::load_from_memory(image_bytes).map_err(|_| NbisError::Nfiq2ComputeFailed(-1))?;
        self.compute_from_luma(&image.to_luma8())
    }

    /// Compute quality from an 8-bit grayscale buffer (same pixels as minutiae extraction).
    pub fn compute_from_luma(&self, gray: &GrayImage) -> Result<Nfiq2Result, NbisError> {
        if self.ctx.is_null() {
            return Err(NbisError::Nfiq2NullContext);
        }

        let (cols, rows) = gray.dimensions();
        if cols == 0 || rows == 0 {
            return Err(NbisError::Nfiq2ComputeFailed(-1));
        }

        let ppi: u16 = 500;
        let mut raw: Nfiq2ResultsT = unsafe { std::mem::zeroed() };

        let rc = unsafe {
            nfiq2wrapper_compute(
                self.ctx,
                gray.as_ptr(),
                gray.len() as c_uint,
                cols as c_uint,
                rows as c_uint,
                ppi as c_ushort,
                &mut raw,
            )
        };
        if rc != 0 {
            unsafe { nfiq2wrapper_free_results(&mut raw) };
            return Err(NbisError::Nfiq2ComputeFailed(rc));
        }

        let result = unsafe { results_from_c(&raw)? };
        unsafe { nfiq2wrapper_free_results(&mut raw) };
        Ok(result)
    }
}

unsafe fn collect_pairs(
    ids_ptr: *const *const c_char,
    vals_ptr: *const f64,
    count: usize,
) -> Result<Vec<Nfiq2Value>, NbisError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if ids_ptr.is_null() || vals_ptr.is_null() {
        return Err(NbisError::Nfiq2ComputeFailed(-1));
    }

    let id_slice = unsafe { std::slice::from_raw_parts(ids_ptr, count) };
    let val_slice = unsafe { std::slice::from_raw_parts(vals_ptr, count) };

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let id_ptr = id_slice[i];
        if id_ptr.is_null() {
            return Err(NbisError::Nfiq2ComputeFailed(-1));
        }
        let s = unsafe { CStr::from_ptr(id_ptr) }
            .to_str()
            .map_err(|_| NbisError::Nfiq2ComputeFailed(-1))?
            .to_string();
        out.push(Nfiq2Value {
            name: s,
            value: val_slice[i],
        });
    }
    Ok(out)
}

unsafe fn results_from_c(raw: &Nfiq2ResultsT) -> Result<Nfiq2Result, NbisError> {
    let actionable_count = raw.actionable_count as usize;
    let feature_count = raw.feature_count as usize;

    let actionable = unsafe {
        collect_pairs(
            raw.actionable_ids,
            raw.actionable_values,
            actionable_count,
        )
    }?;
    let features =
        unsafe { collect_pairs(raw.feature_ids, raw.feature_values, feature_count) }?;

    Ok(Nfiq2Result {
        score: raw.score,
        actionable,
        features,
    })
}

impl Drop for Nfiq2 {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe { nfiq2wrapper_destroy(self.ctx) };
            self.ctx = ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nfiq2() {
        let nfiq = new_nfiq2().expect("failed to create wrapper");

        let expected_scores = vec![54, 45, 53, 52, 57];
        let input_images = vec![
            "ext/NFIQ2-2.3.0/examples/images/SFinGe_Test01.pgm",
            "ext/NFIQ2-2.3.0/examples/images/SFinGe_Test02.pgm",
            "ext/NFIQ2-2.3.0/examples/images/SFinGe_Test03.pgm",
            "ext/NFIQ2-2.3.0/examples/images/SFinGe_Test04.pgm",
            "ext/NFIQ2-2.3.0/examples/images/SFinGe_Test05.pgm",
        ];

        for (i, img_path) in input_images.iter().enumerate() {
            let img_bytes = std::fs::read(img_path).expect("failed to read test image");
            let res = nfiq.compute(&img_bytes).expect("compute failed");
            assert_eq!(res.score, expected_scores[i]);
        }
    }
}
