//! RAII cleanup for NBIS `get_minutiae` output pointers (always freed on every exit path).

use std::{
    os::raw::{c_int, c_uchar},
    ptr::null_mut,
};

use crate::ffi_nbis::{free, free_minutiae, MINUTIAE};

/// Owns all heap pointers returned by `get_minutiae` until this value is dropped.
pub(crate) struct MindtctOutputs {
    pub ominutiae: *mut MINUTIAE,
    pub oquality_map: *mut c_int,
    pub odirection_map: *mut c_int,
    pub olow_contrast_map: *mut c_int,
    pub olow_flow_map: *mut c_int,
    pub ohigh_curve_map: *mut c_int,
    pub obdata: *mut c_uchar,
}

impl MindtctOutputs {
    pub const fn empty() -> Self {
        Self {
            ominutiae: null_mut(),
            oquality_map: null_mut(),
            odirection_map: null_mut(),
            olow_contrast_map: null_mut(),
            olow_flow_map: null_mut(),
            ohigh_curve_map: null_mut(),
            obdata: null_mut(),
        }
    }
}

impl Drop for MindtctOutputs {
    fn drop(&mut self) {
        unsafe {
            if !self.oquality_map.is_null() {
                free(self.oquality_map.cast());
                self.oquality_map = null_mut();
            }
            if !self.odirection_map.is_null() {
                free(self.odirection_map.cast());
                self.odirection_map = null_mut();
            }
            if !self.olow_contrast_map.is_null() {
                free(self.olow_contrast_map.cast());
                self.olow_contrast_map = null_mut();
            }
            if !self.olow_flow_map.is_null() {
                free(self.olow_flow_map.cast());
                self.olow_flow_map = null_mut();
            }
            if !self.ohigh_curve_map.is_null() {
                free(self.ohigh_curve_map.cast());
                self.ohigh_curve_map = null_mut();
            }
            if !self.obdata.is_null() {
                free(self.obdata.cast());
                self.obdata = null_mut();
            }
            if !self.ominutiae.is_null() {
                free_minutiae(self.ominutiae);
                self.ominutiae = null_mut();
            }
        }
    }
}
