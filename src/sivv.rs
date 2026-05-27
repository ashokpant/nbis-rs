use std::ffi::CStr;
use std::os::raw::{c_int, c_uchar};

use crate::errors::NbisError;
use crate::ffi_nbis::{sivv_ffi_free_bytes, sivv_ffi_from_bytes, CPoint2i};
use crate::structs::SIVVResult;

pub(crate) fn is_fingerprint(result: &SIVVResult) -> bool {
    let max_peak_freq = 0.15;
    let peak_height_threshold = 0.02;
    result.peak_frequency < max_peak_freq && result.power_diff > peak_height_threshold
}

fn validate_image_ptr(data: *const u8, width: c_int, height: c_int) -> Result<(), NbisError> {
    if data.is_null() {
        return Err(NbisError::GenericError("null image buffer".into()));
    }
    if width <= 0 || height <= 0 {
        return Err(NbisError::GenericError(format!(
            "invalid image dimensions: {width}x{height}"
        )));
    }
    Ok(())
}

pub(crate) fn find_fingerprint_center(
    data: *const u8,
    width: c_int,
    height: c_int,
) -> Result<(CPoint2i, (i32, i32, i32, i32)), NbisError> {
    validate_image_ptr(data, width, height)?;

    let mut xbound_min: c_int = 0;
    let mut xbound_max: c_int = 0;
    let mut ybound_min: c_int = width;
    let mut ybound_max: c_int = height;

    let result = unsafe {
        crate::ffi_nbis::find_fingerprint_center_morph_c(
            data,
            width,
            height,
            &mut xbound_min,
            &mut xbound_max,
            &mut ybound_min,
            &mut ybound_max,
        )
    };

    Ok((
        result,
        (xbound_min, xbound_max, ybound_min, ybound_max),
    ))
}

pub(crate) fn sivv(image: *mut c_uchar, width: i32, height: i32) -> Result<SIVVResult, NbisError> {
    validate_image_ptr(image.cast(), width as c_int, height as c_int)?;

    let ptr = unsafe {
        sivv_ffi_from_bytes(
            image,
            width as c_int,
            height as c_int,
        )
    };
    if ptr.is_null() {
        return Err(NbisError::GenericError("SIVV native call failed".into()));
    }

    let str_result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
    unsafe { sivv_ffi_free_bytes(ptr) };

    let parts: Vec<&str> = str_result.split(',').map(str::trim).collect();
    if parts.len() != 7 {
        return Err(NbisError::GenericError(
            "Invalid SIVV result format".to_string(),
        ));
    }

    let parse_f64 = |s: &str| -> Result<f64, NbisError> {
        s.parse()
            .map_err(|_| NbisError::GenericError(format!("invalid SIVV field: {s}")))
    };
    let parse_i32 = |s: &str| -> Result<i32, NbisError> {
        s.parse()
            .map_err(|_| NbisError::GenericError(format!("invalid SIVV field: {s}")))
    };

    Ok(SIVVResult {
        largest_pvp_index: parse_i32(parts[0])?,
        total_pvps: parse_i32(parts[1])?,
        power_diff: parse_f64(parts[2])?,
        freq_diff: parse_f64(parts[3])?,
        slope: parse_f64(parts[4])?,
        center_frequency: parse_f64(parts[5])?,
        peak_frequency: parse_f64(parts[6])?,
    })
}
