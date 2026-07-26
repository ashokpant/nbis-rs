use std::{os::raw::c_int, sync::Mutex};

use image::{DynamicImage, GrayImage, Rgb};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut},
    rect::Rect,
};

use crate::{
    consts::MM_PER_INCH,
    crash_guard::{with_native_crash_guard, CrashContext},
    ffi_nbis::{get_minutiae, DEFAULT_BOZORTH_MINUTIAE, LFSPARMS, MINUTIA, MINUTIAE},
    image_limits::{nfiq2_image_ok, validate_extract_dims},
    imutils::{draw_arrow_with_head, png_bytes_from_rgb},
    mindtct_guard::MindtctOutputs,
    minutia::{Minutia, MinutiaKind},
    native_guard::with_extract_lock,
    nfiq2_api::{new_nfiq2, Nfiq2},
    sivv::{find_fingerprint_center, is_fingerprint, sivv},
    structs::NbisExtractorSettings,
    Minutiae, NbisError, Nfiq2Result, Point, ROI,
};

/// Maximum minutiae count accepted from NBIS C structures (DoS guard).
const MAX_MINUTIAE_FROM_C: usize = 10_000;

fn empty_nfiq2() -> Nfiq2Result {
    Nfiq2Result {
        score: 0,
        actionable: Vec::new(),
        features: Vec::new(),
    }
}

#[derive(Debug, uniffi::Object)]
pub struct NbisExtractor {
    settings: NbisExtractorSettings,
    /// Lazily created when `compute_nfiq2` is enabled (NFIQ2 is heavy / crash-prone).
    nfiq2: Mutex<Option<Nfiq2>>,
}

#[uniffi::export]
pub fn new_nbis_extractor(settings: NbisExtractorSettings) -> Result<NbisExtractor, NbisError> {
    NbisExtractor::new(settings)
}

impl NbisExtractor {
    pub fn new(settings: NbisExtractorSettings) -> Result<Self, NbisError> {
        // Do not load NFIQ2 unless requested — avoids model init cost and crash surface.
        let nfiq2 = if settings.compute_nfiq2 {
            Some(new_nfiq2()?)
        } else {
            None
        };
        Ok(NbisExtractor {
            settings,
            nfiq2: Mutex::new(nfiq2),
        })
    }

    fn ensure_nfiq2(&self) -> Result<std::sync::MutexGuard<'_, Option<Nfiq2>>, NbisError> {
        let mut guard = self
            .nfiq2
            .lock()
            .map_err(|_| NbisError::GenericError("NFIQ2 lock poisoned".into()))?;
        if guard.is_none() {
            *guard = Some(new_nfiq2()?);
        }
        Ok(guard)
    }

    /// Soft-fail NFIQ2: never abort extract; return score 0 on error/crash/tiny image.
    fn compute_nfiq2_soft(&self, gray: &GrayImage) -> Nfiq2Result {
        let (w, h) = gray.dimensions();
        if !nfiq2_image_ok(w, h) {
            return empty_nfiq2();
        }
        let mut slot = match self.ensure_nfiq2() {
            Ok(g) => g,
            Err(_) => return empty_nfiq2(),
        };
        let Some(nfiq) = slot.as_mut() else {
            return empty_nfiq2();
        };
        match with_native_crash_guard(CrashContext::Nfiq2, || nfiq.compute_from_luma(gray)) {
            Ok(Ok(q)) => q,
            Ok(Err(_)) | Err(_) => empty_nfiq2(),
        }
    }
}

#[uniffi::export]
impl NbisExtractor {
    pub fn settings(&self) -> NbisExtractorSettings {
        self.settings.clone()
    }

    pub fn load_iso_19794_2_2011(&self, template_bytes: &[u8]) -> Result<Minutiae, NbisError> {
        crate::encoding::load_iso_19794_2_2011(template_bytes)
    }

    /// Bozorth3 score for two ISO/IEC 19794-2 templates (encodes/decodes 2011; reads legacy 2005).
    pub fn compare_iso_19794_2_2011(
        &self,
        probe_template: &[u8],
        gallery_template: &[u8],
    ) -> Result<i32, NbisError> {
        crate::encoding::compare_iso_19794_2_2011(probe_template, gallery_template)
    }

    /// Parallel 1:N Bozorth scores for one probe vs many ISO galleries.
    ///
    /// Thread count: `NBIS_BOZORTH_THREADS` or `min(n_cpus, 8)`. Safe to call concurrently
    /// with other matches (Bozorth is thread-local); still serialize extracts separately.
    pub fn compare_iso_19794_2_2011_batch(
        &self,
        probe_template: &[u8],
        gallery_templates: Vec<Vec<u8>>,
    ) -> Result<Vec<i32>, NbisError> {
        crate::encoding::compare_iso_19794_2_2011_batch(probe_template, &gallery_templates)
    }

    /// Load ISO/IEC 19794-2:2005 templates (and auto-detect 2011 when applicable).
    pub fn load_iso_19794_2_2005(&self, template_bytes: &[u8]) -> Result<Minutiae, NbisError> {
        crate::encoding::load_iso_19794_2_2005(template_bytes)
    }

    pub fn annotate_minutiae_from_image_file(&self, path: &str) -> Result<Vec<u8>, NbisError> {
        let image = std::fs::read(path).map_err(|_| NbisError::FileReadError(path.to_string()))?;
        self.annotate_minutiae(&image)
    }

    pub fn annotate_minutiae(&self, image: &[u8]) -> Result<Vec<u8>, NbisError> {
        let mut image_rgb = match image::load_from_memory(image) {
            Ok(img) => match img {
                image::DynamicImage::ImageRgb8(rgb) => rgb,
                other => other.to_rgb8(),
            },
            Err(_) => return Err(NbisError::ImageLoadError),
        };

        let minutiae = self.extract_minutiae(image)?;

        let img_w = image_rgb.width();
        let img_h = image_rgb.height();

        let square_radius = 2;
        let circle_radius = 2;

        for m in minutiae.inner.iter() {
            let x = m.x;
            let y = m.y;
            if x >= 0 && y >= 0 && (x as u32) < img_w && (y as u32) < img_h {
                match m.kind {
                    MinutiaKind::RidgeEnding => {
                        let rect = Rect::at(x - square_radius, y - square_radius).of_size(
                            (square_radius * 2 + 1) as u32,
                            (square_radius * 2 + 1) as u32,
                        );
                        draw_filled_rect_mut(&mut image_rgb, rect, Rgb([255, 0, 0]));
                    }
                    MinutiaKind::Bifurcation => {
                        draw_filled_circle_mut(
                            &mut image_rgb,
                            (x, y),
                            circle_radius,
                            Rgb([0, 0, 255]),
                        );
                    }
                }

                let color = if m.kind == MinutiaKind::RidgeEnding {
                    Rgb([255, 0, 0])
                } else {
                    Rgb([0, 0, 255])
                };

                draw_arrow_with_head(
                    &mut image_rgb,
                    (x as f32, y as f32),
                    m.angle() as f32,
                    15.0,
                    6.0,
                    color,
                );
            }
        }

        png_bytes_from_rgb(&image_rgb).map_err(|_| NbisError::ImageLoadError)
    }

    pub fn extract_minutiae_from_image_file(&self, file_path: &str) -> Result<Minutiae, NbisError> {
        let image_bytes = std::fs::read(file_path)
            .map_err(|_| NbisError::FileReadError(file_path.to_string()))?;
        self.extract_minutiae(&image_bytes)
    }

    pub fn extract_minutiae(&self, image_bytes: &[u8]) -> Result<Minutiae, NbisError> {
        // Image decode is pure Rust — keep it outside the extract lock.
        let ppi = self.settings.ppi.unwrap_or(500.0);
        let image = image::load_from_memory(image_bytes).map_err(|_| NbisError::ImageLoadError)?;
        let gray: GrayImage = match image {
            DynamicImage::ImageLuma8(buf) => buf,
            _ => image.to_luma8(),
        };
        let (iw, ih) = gray.dimensions();
        let gray_buf = gray.into_raw();

        with_extract_lock(|| self.extract_minutiae_native(gray_buf, iw, ih, ppi))
    }
}

impl NbisExtractor {
    /// mindtct / SIVV / NFIQ2 — caller must hold the extract lock. Not UniFFI-exported.
    fn extract_minutiae_native(
        &self,
        mut gray_buf: Vec<u8>,
        iw: u32,
        ih: u32,
        ppi: f64,
    ) -> Result<Minutiae, NbisError> {
        validate_extract_dims(iw, ih, gray_buf.len())?;

        if self.settings.check_fingerprint {
            let sivv_result = with_native_crash_guard(CrashContext::Sivv, || {
                sivv(gray_buf.as_mut_ptr(), iw as i32, ih as i32)
            })
            .map_err(|c| NbisError::NativeCrash(c.message()))??;
            if !is_fingerprint(&sivv_result) {
                return Ok(Minutiae::new(Vec::new(), iw, ih, empty_nfiq2(), None));
            }
        }

        let roi = if self.settings.get_center {
            let center = with_native_crash_guard(CrashContext::Sivv, || {
                find_fingerprint_center(gray_buf.as_ptr(), iw as c_int, ih as c_int)
            })
            .map_err(|c| NbisError::NativeCrash(c.message()))??;

            Some(ROI {
                x1: center.1 .0,
                x2: center.1 .1,
                y1: center.1 .2,
                y2: center.1 .3,
                center: Point {
                    x: center.0.x,
                    y: center.0.y,
                },
            })
        } else {
            None
        };

        let mut maps = MindtctOutputs::empty();
        let mut map_w: c_int = 0;
        let mut map_h: c_int = 0;
        let mut obw: c_int = 0;
        let mut obh: c_int = 0;
        let mut obd: c_int = 0;
        let ppmm = ppi / MM_PER_INCH;

        let rc = with_native_crash_guard(CrashContext::Mindtct, || unsafe {
            unsafe extern "C" {
                static lfsparms_V2: LFSPARMS;
            }
            get_minutiae(
                &mut maps.ominutiae,
                &mut maps.oquality_map,
                &mut maps.odirection_map,
                &mut maps.olow_contrast_map,
                &mut maps.olow_flow_map,
                &mut maps.ohigh_curve_map,
                &mut map_w,
                &mut map_h,
                &mut maps.obdata,
                &mut obw,
                &mut obh,
                &mut obd,
                gray_buf.as_mut_ptr(),
                iw as c_int,
                ih as c_int,
                8,
                ppmm,
                &lfsparms_V2 as *const _,
            )
        })
        .map_err(|c| NbisError::NativeCrash(c.message()))?;

        if rc != 0 {
            return Err(NbisError::UnexpectedError(rc as i64));
        }

        let gray_for_nfiq =
            GrayImage::from_raw(iw, ih, gray_buf).ok_or(NbisError::ImageLoadError)?;

        // Soft-fail: NFIQ2 must never take down extract / the host process.
        let quality = if self.settings.compute_nfiq2 {
            self.compute_nfiq2_soft(&gray_for_nfiq)
        } else {
            empty_nfiq2()
        };

        let minutiae_vec = parse_minutiae_from_c(maps.ominutiae)?;
        let mut minutiae_obj = Minutiae::new(minutiae_vec, iw, ih, quality, roi);

        if self.settings.min_quality > 0.0 {
            minutiae_obj
                .inner
                .retain(|m| m.reliability >= self.settings.min_quality);
        }

        if minutiae_obj.inner.len() > DEFAULT_BOZORTH_MINUTIAE {
            minutiae_obj.inner.sort_by(|a, b| {
                b.reliability
                    .partial_cmp(&a.reliability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            minutiae_obj.inner.truncate(DEFAULT_BOZORTH_MINUTIAE);
        }

        minutiae_obj
            .inner
            .sort_by(|a, b| a.x.cmp(&b.x).then(a.y.cmp(&b.y)));

        Ok(minutiae_obj)
    }
}

fn parse_minutiae_from_c(ominutiae: *mut MINUTIAE) -> Result<Vec<Minutia>, NbisError> {
    if ominutiae.is_null() {
        return Err(NbisError::InvalidMinutiaeData);
    }

    let mset = unsafe { &*ominutiae };
    let num = mset.num;
    if num < 0 {
        return Err(NbisError::InvalidMinutiaeData);
    }
    let count = num as usize;
    if count > MAX_MINUTIAE_FROM_C {
        return Err(NbisError::InvalidMinutiaeData);
    }
    if mset.alloc > 0 && num > mset.alloc {
        return Err(NbisError::InvalidMinutiaeData);
    }
    if count > 0 && mset.list.is_null() {
        return Err(NbisError::InvalidMinutiaeData);
    }

    let raw: &[ *mut MINUTIA] = if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(mset.list, count) }
    };

    let mut out = Vec::with_capacity(count);
    for ptr in raw {
        if ptr.is_null() {
            return Err(NbisError::InvalidMinutiaeData);
        }
        let m = unsafe { &**ptr };
        let kind = match m.r#type {
            0 => MinutiaKind::Bifurcation,
            1 => MinutiaKind::RidgeEnding,
            _ => return Err(NbisError::InvalidMinutiaeData),
        };
        out.push(Minutia {
            x: m.x,
            y: m.y,
            direction: m.direction,
            reliability: m.reliability,
            kind,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use crate::ffi_nbis::DEFAULT_BOZORTH_MINUTIAE;

    use super::*;
    use std::fs;

    #[test]
    fn list_matches_get_fields() {
        let extractor = new_nbis_extractor(NbisExtractorSettings {
            compute_nfiq2: false,
            ..NbisExtractorSettings::default()
        })
        .unwrap();
        let tpl = extractor
            .extract_minutiae(&fs::read("test_data/p1/p1_1.png").unwrap())
            .unwrap();
        let via_list = tpl.list();
        let via_get = tpl.get();
        assert_eq!(via_list.len(), via_get.len());
        assert!(!via_list.is_empty());
        for (a, b) in via_list.iter().zip(via_get.iter()) {
            assert_eq!(a.x, b.x());
            assert_eq!(a.y, b.y());
            assert_eq!(a.kind, b.kind());
            assert!((a.reliability - b.reliability()).abs() < 1e-9);
            assert!((a.angle - b.angle()).abs() < 1e-9);
        }
    }

    #[test]
    fn concurrent_extract_is_safe() {
        use std::sync::Arc;
        use std::thread;

        let image = fs::read("test_data/p1/p1_1.png").unwrap();
        let extractor = Arc::new(
            new_nbis_extractor(NbisExtractorSettings {
                compute_nfiq2: false,
                ..NbisExtractorSettings::default()
            })
            .unwrap(),
        );

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let ex = Arc::clone(&extractor);
                let bytes = image.clone();
                thread::spawn(move || {
                    let tpl = ex.extract_minutiae(&bytes).unwrap();
                    assert!(!tpl.inner.is_empty());
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("extract thread panicked");
        }
    }

    #[test]
    fn concurrent_compare_is_parallel_and_correct() {
        use std::sync::Arc;
        use std::thread;

        let extractor = new_nbis_extractor(NbisExtractorSettings {
            compute_nfiq2: false,
            ..NbisExtractorSettings::default()
        })
        .unwrap();
        let p1 = extractor
            .extract_minutiae(&fs::read("test_data/p1/p1_1.png").unwrap())
            .unwrap();
        let p2 = extractor
            .extract_minutiae(&fs::read("test_data/p1/p1_2.png").unwrap())
            .unwrap();
        let p3 = extractor
            .extract_minutiae(&fs::read("test_data/p2/p2_1.png").unwrap())
            .unwrap();

        let serial_12 = p1.compare(&p2);
        let serial_13 = p1.compare(&p3);
        let serial_23 = p2.compare(&p3);

        let p1 = Arc::new(p1);
        let p2 = Arc::new(p2);
        let p3 = Arc::new(p3);

        let handles: Vec<_> = (0..16)
            .map(|i| {
                let a = Arc::clone(&p1);
                let b = Arc::clone(&p2);
                let c = Arc::clone(&p3);
                thread::spawn(move || {
                    let s12 = a.compare(&b);
                    let s13 = a.compare(&c);
                    let s23 = b.compare(&c);
                    assert_eq!(s12, serial_12, "thread {i} p1↔p2");
                    assert_eq!(s13, serial_13, "thread {i} p1↔p3");
                    assert_eq!(s23, serial_23, "thread {i} p2↔p3");
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("compare thread panicked");
        }
    }

    #[test]
    fn extract_and_match_overlap_is_safe() {
        use std::sync::Arc;
        use std::thread;

        let image = fs::read("test_data/p1/p1_1.png").unwrap();
        let extractor = Arc::new(
            new_nbis_extractor(NbisExtractorSettings {
                compute_nfiq2: false,
                ..NbisExtractorSettings::default()
            })
            .unwrap(),
        );
        let probe = Arc::new(
            extractor
                .extract_minutiae(&fs::read("test_data/p1/p1_1.png").unwrap())
                .unwrap(),
        );
        let gallery = Arc::new(
            extractor
                .extract_minutiae(&fs::read("test_data/p1/p1_2.png").unwrap())
                .unwrap(),
        );
        let expected = probe.compare(&gallery);

        let mut handles = Vec::new();
        for _ in 0..4 {
            let ex = Arc::clone(&extractor);
            let bytes = image.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..3 {
                    let tpl = ex.extract_minutiae(&bytes).unwrap();
                    assert!(!tpl.inner.is_empty());
                }
            }));
        }
        for _ in 0..8 {
            let p = Arc::clone(&probe);
            let g = Arc::clone(&gallery);
            handles.push(thread::spawn(move || {
                for _ in 0..20 {
                    assert_eq!(p.compare(&g), expected);
                }
            }));
        }

        for handle in handles {
            handle.join().expect("extract‖match thread panicked");
        }
    }

    #[test]
    fn batch_compare_matches_serial() {
        let extractor = new_nbis_extractor(NbisExtractorSettings {
            compute_nfiq2: false,
            ..NbisExtractorSettings::default()
        })
        .unwrap();
        let a = extractor
            .extract_minutiae(&fs::read("test_data/p1/p1_1.png").unwrap())
            .unwrap();
        let b = extractor
            .extract_minutiae(&fs::read("test_data/p1/p1_2.png").unwrap())
            .unwrap();
        let c = extractor
            .extract_minutiae(&fs::read("test_data/p2/p2_1.png").unwrap())
            .unwrap();
        let probe = a.to_iso_19794_2_2011().unwrap();
        let galleries = vec![
            b.to_iso_19794_2_2011().unwrap(),
            c.to_iso_19794_2_2011().unwrap(),
            a.to_iso_19794_2_2011().unwrap(),
        ];
        let serial: Vec<i32> = galleries
            .iter()
            .map(|g| extractor.compare_iso_19794_2_2011(&probe, g).unwrap())
            .collect();
        let batch = extractor
            .compare_iso_19794_2_2011_batch(&probe, galleries)
            .unwrap();
        assert_eq!(batch, serial);
    }

    #[test]
    fn test_match() {
        let p_1 = fs::read("test_data/p1/p1_1.png").unwrap();
        let p1_2 = fs::read("test_data/p1/p1_2.png").unwrap();
        let p1_3 = fs::read("test_data/p1/p1_3.png").unwrap();

        let extractor = new_nbis_extractor(NbisExtractorSettings::default()).unwrap();

        let res1 = extractor.extract_minutiae(&p_1).unwrap();
        let res2 = extractor.extract_minutiae(&p1_2).unwrap();
        let res3 = extractor.extract_minutiae(&p1_3).unwrap();
        let score1 = res1.compare(&res2);

        assert_eq!(score1, res2.compare(&res1), "Scores should be symmetric");

        let score2 = res1.compare(&res3);
        let score3 = res2.compare(&res3);
        assert!(score1 > 50);
        assert!(score2 > 50);
        assert!(score3 > 50);

        let p2_1 = fs::read("test_data/p2/p2_1.png").unwrap();
        let p2_2 = fs::read("test_data/p2/p2_2.png").unwrap();
        let p2_3 = fs::read("test_data/p2/p2_3.png").unwrap();

        let res4 = extractor.extract_minutiae(&p2_1).unwrap();
        let res5 = extractor.extract_minutiae(&p2_2).unwrap();
        let res6 = extractor.extract_minutiae(&p2_3).unwrap();
        let score4 = res4.compare(&res5);
        let score5 = res4.compare(&res6);
        let score6 = res5.compare(&res6);

        assert!(score4 > 50);
        assert!(score5 > 50);
        assert!(score6 > 50);

        let score7 = res1.compare(&res4);
        let score8 = res1.compare(&res5);
        let score9 = res1.compare(&res6);

        assert!(score7 < 50);
        assert!(score8 < 50);
        assert!(score9 < 50);
    }

    #[test]
    fn test_encode_to_iso() {
        let extractor = new_nbis_extractor(NbisExtractorSettings::default()).unwrap();
        let bryanc_1 = fs::read("test_data/p1/p1_1.png").unwrap();
        let res = extractor.extract_minutiae(&bryanc_1).unwrap();
        let encoded = res.to_iso_19794_2_2011().unwrap();
        assert!(!encoded.is_empty());
        assert_eq!(&encoded[4..8], b"030\0");

        let minutiae = extractor.load_iso_19794_2_2011(&encoded).unwrap();
        let score_via_compare_iso = extractor
            .compare_iso_19794_2_2011(&encoded, &encoded)
            .unwrap();
        assert_eq!(score_via_compare_iso, res.compare(&minutiae));

        assert_eq!(res.quality().score, minutiae.quality().score);
        assert_eq!(minutiae.inner.len(), res.inner.len());
        assert_eq!(minutiae.img_w, res.img_w);
        assert_eq!(minutiae.img_h, res.img_h);

        for (m1, m2) in res.inner.iter().zip(minutiae.inner.iter()) {
            assert_eq!(m1.x, m2.x);
            assert_eq!(m1.y, m2.y);
            assert_eq!(m1.direction, m2.direction);
            assert!((m1.reliability - m2.reliability).abs() < 1e-1);
            assert_eq!(m1.kind, m2.kind);
        }

        let mut many_minutiae = res.inner.clone();
        for i in 0..300 {
            many_minutiae.push(Minutia {
                x: i as i32,
                y: i as i32,
                direction: 0,
                reliability: 0.0,
                kind: MinutiaKind::RidgeEnding,
            });
        }

        let many_res = Minutiae::new(many_minutiae, res.img_w, res.img_h, res.nfiq, None);
        let many_encoded = many_res.to_iso_19794_2_2011().unwrap();
        assert!(!many_encoded.is_empty());

        let many_minutiae_decoded = extractor.load_iso_19794_2_2011(&many_encoded).unwrap();
        assert_eq!(many_minutiae_decoded.inner.len(), DEFAULT_BOZORTH_MINUTIAE);

        let bryanc_2 = fs::read("test_data/p1/p1_2.png").unwrap();
        let r1 = extractor.extract_minutiae(&bryanc_1).unwrap();
        let r2 = extractor.extract_minutiae(&bryanc_2).unwrap();
        let e1 = r1.to_iso_19794_2_2011().unwrap();
        let e2 = r2.to_iso_19794_2_2011().unwrap();
        let reloaded_e1 = extractor.load_iso_19794_2_2011(&e1).unwrap();
        let reloaded_e2 = extractor.load_iso_19794_2_2011(&e2).unwrap();

        let s1 = r1.compare(&r2);
        let s2 = r1.compare(&reloaded_e2);
        let s3 = reloaded_e1.compare(&r2);
        let s4 = reloaded_e1.compare(&reloaded_e2);

        assert_eq!(r1.inner.len(), reloaded_e1.inner.len());
        assert_eq!(s1, s2);
        assert_eq!(s1, s3);
        assert_eq!(s2, s4);
        assert_eq!(s3, s4);
        assert_eq!(s1, s4);
    }

    #[test]
    fn test_nfiq() {
        let extractor = new_nbis_extractor(NbisExtractorSettings::default()).unwrap();
        let p1_1 = fs::read("test_data/p1/p1_1.png").unwrap();
        let res = extractor.extract_minutiae(&p1_1).unwrap();
        assert!(res.quality().score > 60);

        let random_image = fs::read("test_data/negative/landscape.jpg").unwrap();
        let extractor = new_nbis_extractor(NbisExtractorSettings {
            min_quality: 0.0,
            get_center: false,
            check_fingerprint: true,
            compute_nfiq2: true,
            ppi: None,
        })
        .unwrap();
        let res2 = extractor.extract_minutiae(&random_image).unwrap();
        assert_eq!(res2.quality().score, 0);

        let random_image = fs::read("test_data/negative/face.jpeg").unwrap();
        let res2 = extractor.extract_minutiae(&random_image).unwrap();
        assert_eq!(res2.quality().score, 0);
    }

    #[test]
    fn test_negative() {
        let extractor = new_nbis_extractor(NbisExtractorSettings::default()).unwrap();
        let res1 = extractor.extract_minutiae_from_image_file("build.rs");
        assert!(res1.is_err());
        match res1 {
            Err(NbisError::ImageLoadError) => {}
            Err(other) => panic!("Expected ImageLoadError but got: {:?}", other),
            Ok(_) => panic!("Expected error but got Ok"),
        }

        let res2 = extractor.extract_minutiae_from_image_file("test_data/negative/x.png");
        assert!(res2.is_err());
        match res2 {
            Err(NbisError::FileReadError(_)) => {}
            Err(other) => panic!("Expected FileReadError but got: {:?}", other),
            Ok(_) => panic!("Expected error but got Ok"),
        }

        let n_2 = fs::read("test_data/negative/varun_square.png").unwrap();
        let extractor = new_nbis_extractor(NbisExtractorSettings {
            min_quality: 0.0,
            get_center: false,
            check_fingerprint: true,
            compute_nfiq2: false,
            ppi: None,
        })
        .unwrap();

        let res1_n_2 = extractor.extract_minutiae(&n_2).unwrap();
        let res2_n_2 = extractor.extract_minutiae(&n_2).unwrap();
        let score_n_2 = res1_n_2.compare(&res2_n_2);
        assert_eq!(score_n_2, 0);
    }

    #[test]
    fn test_roi() {
        let p1_1 = fs::read("test_data/p1/p1_1.png").unwrap();
        let extractor = new_nbis_extractor(NbisExtractorSettings {
            min_quality: 0.0,
            get_center: true,
            check_fingerprint: false,
            compute_nfiq2: false,
            ppi: None,
        })
        .unwrap();
        let res = extractor.extract_minutiae(&p1_1).unwrap();
        assert!(res.roi().is_some());
        let roi = res.roi().unwrap();

        assert!(roi.x1 < roi.x2 && roi.y1 < roi.y2);
        assert_eq!(roi.x1, 0);
        assert_eq!(roi.y1, 96);
        assert_eq!(roi.x2, 382);
        assert_eq!(roi.y2, 496);
        assert_eq!(roi.center.x, 182);
        assert_eq!(roi.center.y, 296);
    }

    #[test]
    fn rejects_tiny_images() {
        let extractor = new_nbis_extractor(NbisExtractorSettings {
            compute_nfiq2: false,
            ..NbisExtractorSettings::default()
        })
        .unwrap();
        // 8x8 gray PNG
        let img = image::GrayImage::from_pixel(8, 8, image::Luma([128u8]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        let err = extractor.extract_minutiae(&bytes).unwrap_err();
        match err {
            NbisError::GenericError(msg) => assert!(msg.contains("too small"), "{msg}"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn extract_without_nfiq2_skips_model_load() {
        let extractor = new_nbis_extractor(NbisExtractorSettings {
            compute_nfiq2: false,
            ..NbisExtractorSettings::default()
        })
        .unwrap();
        assert!(extractor.nfiq2.lock().unwrap().is_none());
        let tpl = extractor
            .extract_minutiae(&fs::read("test_data/p1/p1_1.png").unwrap())
            .unwrap();
        assert!(!tpl.inner.is_empty());
        assert_eq!(tpl.quality().score, 0);
        assert!(extractor.nfiq2.lock().unwrap().is_none());
    }
}
