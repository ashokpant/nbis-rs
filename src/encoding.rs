use std::cmp::Ordering;

use crate::{
    bozorth::MinutiaeSet, consts::MM_PER_INCH, consts::NUM_DIRECTIONS,
    ffi_nbis::DEFAULT_BOZORTH_MINUTIAE, Minutia, MinutiaKind, Minutiae, NbisError, Nfiq2Result,
};

const ISO_2011_HEADER_LEN: usize = 15;
const ISO_2011_MAGIC: &[u8; 4] = b"FMR\0";
const ISO_2011_VERSION: &[u8; 4] = b"030\0";
const ISO_2005_VERSION: &[u8; 8] = b"FMR\0 20\0";

/// NIST vendor ID in IBIA registry (used for NFIQ quality algorithm).
const QVENDOR_NIST: u16 = 0x000f;
/// Quality algorithm ID (0 = unspecified per ISO 19794-1).
const QALGO_UNSPECIFIED: u16 = 0;

/// Quantise an angle (degrees) into the 8-bit ISO/IEC 19794-2 orientation unit.
///
/// The ISO unit represents 360 ° / 256 ≈ 1.40625 ° per code value.
/// Any input (positive or negative) is first wrapped into the range [0, 360).
#[inline]
pub(crate) fn encode_iso_angle(angle_deg: f64) -> u8 {
    let angle_deg = 90.0 - angle_deg * 11.25;
    const ISO_STEP: f64 = 360.0 / 256.0;
    let norm = angle_deg.rem_euclid(360.0);
    let quantised = (norm / ISO_STEP).round() as i32 & 0xFF;
    quantised as u8
}

/// Encode a single minutia into its 6-byte ISO/IEC 19794-2:2011 representation.
#[inline]
pub(crate) fn encode_minutia(m: &Minutia) -> Result<[u8; 6], NbisError> {
    if m.x < 0 || m.y < 0 || m.x > i32::from(u16::MAX) || m.y > i32::from(u16::MAX) {
        return Err(NbisError::CoordinateOutOfRange(format!(
            "x={}, y={}",
            m.x, m.y
        )));
    }
    let min_type = if m.kind == MinutiaKind::Bifurcation {
        2u8
    } else {
        1u8
    };
    let angle = encode_iso_angle(m.direction as f64);
    let quality = (m.reliability * 100.0).round().clamp(0.0, 100.0) as u8;
    let x = m.x as u16;
    let y = m.y as u16;
    let mut bytes = [0u8; 6];

    bytes[1] = (x & 0x00FF) as u8;
    bytes[0] = ((x >> 8) as u8) + (min_type << 6);
    bytes[2] = (y >> 8) as u8;
    bytes[3] = (y & 0x00FF) as u8;
    bytes[4] = angle;
    bytes[5] = quality;

    Ok(bytes)
}

fn nist_xyt(minutiae: &Minutiae, minutia: &Minutia) -> (i32, i32, i32) {
    let x = minutia.x;
    let y = minutiae.img_h as i32 - minutia.y;
    let degrees_per_unit = 180.0 / NUM_DIRECTIONS;
    let t = (270 - ((minutia.direction as f64 * degrees_per_unit).round() as i32)) % 360;
    let t = if t < 0 { t + 360 } else { t };
    (x, y, t)
}

/// Convert this result into a Bozorth‑ready [`MinutiaeSet`].
pub(crate) fn to_nist_xyt_set(minutiae: &Minutiae) -> MinutiaeSet {
    let len = minutiae.inner.len();
    let mut xs = Vec::with_capacity(len);
    let mut ys = Vec::with_capacity(len);
    let mut ts = Vec::with_capacity(len);

    for m in &minutiae.inner {
        let (ox, oy, ot) = nist_xyt(minutiae, m);
        xs.push(ox);
        ys.push(oy);
        ts.push(ot);
    }

    MinutiaeSet { xs, ys, theta: ts }
}

#[inline]
pub(crate) fn decode_iso_angle(iso_code: u8) -> i32 {
    const ISO_STEP: f64 = 360.0 / 256.0;
    const NIST_STEP: f64 = 11.25;

    let iso_deg = iso_code as f64 * ISO_STEP;
    let nist_unit = (90.0 - iso_deg).rem_euclid(360.0) / NIST_STEP;
    (nist_unit.round() as u32 & 0x1F) as i32
}

/// Decode the 6-byte minutia record used by `encode_minutia`.
#[inline]
pub(crate) fn decode_minutia(bytes: &[u8; 6], min_quality_scale: MinQualityScale) -> Minutia {
    let x = (u16::from(bytes[0] & 0x3F) << 8) | u16::from(bytes[1]);
    let y = (u16::from(bytes[2]) << 8) | u16::from(bytes[3]);
    let direction = decode_iso_angle(bytes[4]);
    let reliability = match min_quality_scale {
        MinQualityScale::ZeroTo100 => f64::from(bytes[5]) / 100.0,
        MinQualityScale::ZeroTo63 => f64::from(bytes[5]) / 63.0,
    };
    let kind = if bytes[0] & 0xC0 == 0x80 {
        MinutiaKind::Bifurcation
    } else {
        MinutiaKind::RidgeEnding
    };

    Minutia {
        x: x as i32,
        y: y as i32,
        direction,
        reliability,
        kind,
    }
}

#[derive(Clone, Copy)]
pub(crate) enum MinQualityScale {
    ZeroTo100,
    ZeroTo63,
}

fn select_minutiae_for_template(minutiae_obj: &Minutiae) -> Result<Vec<Minutia>, NbisError> {
    let mut minutiae = minutiae_obj.inner.clone();

    if minutiae.len() > DEFAULT_BOZORTH_MINUTIAE {
        minutiae.sort_by(|a, b| {
            b.reliability
                .partial_cmp(&a.reliability)
                .unwrap_or(Ordering::Equal)
        });
        minutiae.truncate(DEFAULT_BOZORTH_MINUTIAE);
    }

    if minutiae.is_empty() {
        return Err(NbisError::InvalidTemplate(
            "ISO 19794-2:2011 requires at least one minutia".into(),
        ));
    }

    Ok(minutiae)
}

fn ppcm_from_ppi(ppi: f64) -> u16 {
    (ppi / MM_PER_INCH).round().clamp(99.0, u16::MAX as f64) as u16
}

fn fingerprint_preamble_len(qcount: u8) -> usize {
    4 + 9 + 1 + 2 + 2 + 1 + usize::from(qcount) * 5 + 1 + 1 + 2 + 2 + 1 + 2 + 2 + 1 + 1
}

/// Convert this set of minutiae into an ISO/IEC 19794-2:2011 template.
pub fn to_iso_19794_2_2011(minutiae_obj: &Minutiae) -> Result<Vec<u8>, NbisError> {
    let minutiae = select_minutiae_for_template(minutiae_obj)?;
    let ppi = 500.0_f64;
    let resolution = ppcm_from_ppi(ppi);
    let width = minutiae_obj.img_w as u16;
    let height = minutiae_obj.img_h as u16;

    const QCOUNT: u8 = 1;
    let fp_preamble = fingerprint_preamble_len(QCOUNT);
    let fp_bytes = fp_preamble + minutiae.len() * 6 + 2;
    let total_bytes = ISO_2011_HEADER_LEN + fp_bytes;

    let total_bytes_u32 =
        u32::try_from(total_bytes).map_err(|_| NbisError::InvalidTemplate("template too large".into()))?;
    let fp_bytes_u32 =
        u32::try_from(fp_bytes).map_err(|_| NbisError::InvalidTemplate("fingerprint block too large".into()))?;

    let mut buf = Vec::with_capacity(total_bytes);

    // HEADER (15 bytes)
    buf.extend_from_slice(ISO_2011_MAGIC);
    buf.extend_from_slice(ISO_2011_VERSION);
    buf.extend_from_slice(&total_bytes_u32.to_be_bytes());
    buf.extend_from_slice(&1u16.to_be_bytes()); // FPCOUNT
    buf.push(0); // HASCERTS

    // FINGERPRINT
    buf.extend_from_slice(&fp_bytes_u32.to_be_bytes());

    // DATETIME — not provided
    buf.extend_from_slice(&[0xff; 9]);

    buf.push(0); // DEVTECH: unknown
    buf.extend_from_slice(&0u16.to_be_bytes()); // DEVVENDOR
    buf.extend_from_slice(&0u16.to_be_bytes()); // DEVID
    buf.push(QCOUNT);

    let fp_quality = minutiae_obj.nfiq.score.min(100) as u8;
    buf.push(fp_quality);
    buf.extend_from_slice(&QVENDOR_NIST.to_be_bytes());
    buf.extend_from_slice(&QALGO_UNSPECIFIED.to_be_bytes());

    buf.push(0); // POSITION: unknown
    buf.push(0); // VIEWOFFSET
    buf.extend_from_slice(&resolution.to_be_bytes());
    buf.extend_from_slice(&resolution.to_be_bytes());
    buf.push(0); // SAMPLETYPE: live plain
    buf.extend_from_slice(&width.to_be_bytes());
    buf.extend_from_slice(&height.to_be_bytes());
    buf.push(0x60); // MINBYTES=6, ENDINGTYPE=0 (valley bifurcation)
    buf.push(minutiae.len() as u8);

    for m in &minutiae {
        buf.extend_from_slice(&encode_minutia(m)?);
    }

    buf.extend_from_slice(&0u16.to_be_bytes()); // EXTBYTES

    debug_assert_eq!(buf.len(), total_bytes);
    Ok(buf)
}

/// Bozorth3 match score from two ISO/IEC 19794-2 templates (2011 or legacy 2005).
pub fn compare_iso_19794_2_2011(probe_template: &[u8], gallery_template: &[u8]) -> Result<i32, NbisError> {
    let probe = load_iso_19794_2_2011(probe_template)?;
    let gallery = load_iso_19794_2_2011(gallery_template)?;
    Ok(probe.compare(&gallery))
}

/// Parallel 1:N Bozorth scores. Thread count from `NBIS_BOZORTH_THREADS` (default `min(n_cpus, 8)`).
pub fn compare_iso_19794_2_2011_batch(
    probe_template: &[u8],
    gallery_templates: &[Vec<u8>],
) -> Result<Vec<i32>, NbisError> {
    let probe = load_iso_19794_2_2011(probe_template)?;
    if gallery_templates.is_empty() {
        return Ok(Vec::new());
    }

    let threads = crate::bozorth_pool::bozorth_thread_count();
    if threads <= 1 || gallery_templates.len() == 1 {
        let mut scores = Vec::with_capacity(gallery_templates.len());
        for g in gallery_templates {
            let score = match load_iso_19794_2_2011(g) {
                Ok(gallery) => probe.compare(&gallery),
                Err(_) => 0,
            };
            scores.push(score);
        }
        return Ok(scores);
    }

    use rayon::prelude::*;

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| NbisError::GenericError(format!("Bozorth thread pool: {e}")))?;

    pool.install(|| {
        gallery_templates
            .par_iter()
            .map(|g| {
                // One bad gallery must not abort the whole 1:N batch.
                match load_iso_19794_2_2011(g) {
                    Ok(gallery) => Ok(probe.compare(&gallery)),
                    Err(_) => Ok(0),
                }
            })
            .collect()
    })
}

/// Loads an ISO/IEC 19794-2:2011 fingerprint template from bytes.
pub fn load_iso_19794_2_2011(template_bytes: &[u8]) -> Result<Minutiae, NbisError> {
    if template_bytes.len() >= 8 && &template_bytes[0..8] == ISO_2005_VERSION {
        return load_iso_19794_2_2005(template_bytes);
    }
    parse_iso_2011_template(template_bytes)
}

/// Loads an ISO/IEC 19794-2:2005 fingerprint template from bytes.
pub fn load_iso_19794_2_2005(template_bytes: &[u8]) -> Result<Minutiae, NbisError> {
    const ISO_HEADER_LENGTH: usize = 26;
    if template_bytes.len() < ISO_HEADER_LENGTH {
        return Err(NbisError::InvalidTemplate(
            "ISO template too short".to_string(),
        ));
    }

    if &template_bytes[0..8] != ISO_2005_VERSION {
        if template_bytes.len() >= ISO_2011_HEADER_LEN
            && &template_bytes[0..4] == ISO_2011_MAGIC
            && &template_bytes[4..8] == ISO_2011_VERSION
        {
            return parse_iso_2011_template(template_bytes);
        }
        return Err(NbisError::InvalidTemplate("Invalid ISO header".to_string()));
    }

    let total_length = u32::from_be_bytes([
        template_bytes[8],
        template_bytes[9],
        template_bytes[10],
        template_bytes[11],
    ]) as usize;
    if total_length != template_bytes.len() {
        return Err(NbisError::InvalidTemplate(
            "Total length mismatch".to_string(),
        ));
    }

    let width = u16::from_be_bytes([template_bytes[14], template_bytes[15]]);
    let height = u16::from_be_bytes([template_bytes[16], template_bytes[17]]);
    let finger_quality = template_bytes[20];
    let num_minutiae = template_bytes[25] as usize;
    let minutiae_start: usize = 26;

    let minutiae = decode_minutiae_records(
        template_bytes,
        minutiae_start,
        num_minutiae,
        6,
        template_bytes.len(),
        MinQualityScale::ZeroTo63,
    )?;

    Ok(Minutiae::new(
        minutiae,
        width as u32,
        height as u32,
        nfiq_from_quality(finger_quality),
        None,
    ))
}

fn parse_iso_2011_template(template_bytes: &[u8]) -> Result<Minutiae, NbisError> {
    if template_bytes.len() < ISO_2011_HEADER_LEN {
        return Err(NbisError::InvalidTemplate(
            "ISO 2011 template too short".into(),
        ));
    }

    if &template_bytes[0..4] != ISO_2011_MAGIC {
        return Err(NbisError::InvalidTemplate("Invalid ISO magic".into()));
    }
    if &template_bytes[4..8] != ISO_2011_VERSION {
        return Err(NbisError::InvalidTemplate(
            "Unsupported ISO 19794-2 version".into(),
        ));
    }

    let total_length = usize_from_be_u32(&template_bytes[8..12])?;
    // Allow trailing padding after the declared template length; reject truncation.
    if total_length > template_bytes.len() {
        return Err(NbisError::InvalidTemplate("Total length mismatch".into()));
    }
    let template_bytes = &template_bytes[..total_length];

    let fp_count = usize::from(u16::from_be_bytes([template_bytes[12], template_bytes[13]]));
    if fp_count != 1 {
        return Err(NbisError::InvalidTemplate(format!(
            "unsupported FPCOUNT: {fp_count}"
        )));
    }

    let fp_start = ISO_2011_HEADER_LEN;
    let fp_bytes = usize_from_be_u32(&template_bytes[fp_start..fp_start + 4])?;
    let mut offset = fp_start + 4;

    let fp_end = fp_start
        .checked_add(fp_bytes)
        .ok_or_else(|| NbisError::InvalidTemplate("fingerprint block overflow".into()))?;
    if fp_end > template_bytes.len() {
        return Err(NbisError::InvalidTemplate(
            "fingerprint block exceeds template".into(),
        ));
    }

    offset += 9; // DATETIME
    offset += 1; // DEVTECH
    offset += 2; // DEVVENDOR
    offset += 2; // DEVID

    if offset >= fp_end {
        return Err(NbisError::InvalidTemplate("truncated fingerprint header".into()));
    }
    let qcount = template_bytes[offset];
    offset += 1;

    let mut finger_quality = 0u8;
    if qcount > 0 {
        let q_end = offset
            .checked_add(usize::from(qcount) * 5)
            .ok_or_else(|| NbisError::InvalidTemplate("quality records overflow".into()))?;
        if q_end > fp_end {
            return Err(NbisError::InvalidTemplate(
                "quality records exceed fingerprint block".into(),
            ));
        }
        finger_quality = template_bytes[offset];
        offset = q_end;
    }

    if offset + 13 > fp_end {
        return Err(NbisError::InvalidTemplate(
            "truncated fingerprint metadata".into(),
        ));
    }

    offset += 1; // POSITION
    offset += 1; // VIEWOFFSET
    offset += 2; // RESOLUTIONX
    offset += 2; // RESOLUTIONY
    offset += 1; // SAMPLETYPE

    let width = u16::from_be_bytes([template_bytes[offset], template_bytes[offset + 1]]);
    offset += 2;
    let height = u16::from_be_bytes([template_bytes[offset], template_bytes[offset + 1]]);
    offset += 2;

    // High nibble = bytes per minutia record (ISO 19794-2); low nibble = ending type.
    let min_bytes = (template_bytes[offset] >> 4) & 0x0f;
    offset += 1;

    let num_minutiae = template_bytes[offset] as usize;
    offset += 1;

    let min_record_len = usize::from(min_bytes);
    // 5/6 are standard; 7/8 appear in vendor/extended templates (extra reserved bytes).
    if !(5..=8).contains(&min_record_len) {
        return Err(NbisError::InvalidTemplate(format!(
            "unsupported MINBYTES: {min_bytes}"
        )));
    }

    // Leave room for EXTBYTES (u16) after the minutiae block when present.
    let minutiae_end_limit = fp_end.saturating_sub(2).max(offset);
    let minutiae = decode_minutiae_records(
        template_bytes,
        offset,
        num_minutiae,
        min_record_len,
        minutiae_end_limit,
        if min_record_len >= 6 {
            MinQualityScale::ZeroTo100
        } else {
            MinQualityScale::ZeroTo63
        },
    )?;

    Ok(Minutiae::new(
        minutiae,
        width as u32,
        height as u32,
        nfiq_from_quality(finger_quality),
        None,
    ))
}

fn decode_minutiae_records(
    template_bytes: &[u8],
    minutiae_start: usize,
    num_minutiae: usize,
    record_len: usize,
    end_limit: usize,
    quality_scale: MinQualityScale,
) -> Result<Vec<Minutia>, NbisError> {
    const MAX_TEMPLATE_MINUTIAE: usize = 512;
    if num_minutiae > MAX_TEMPLATE_MINUTIAE {
        return Err(NbisError::InvalidTemplate(format!(
            "too many minutiae: {num_minutiae}"
        )));
    }
    if record_len == 0 {
        return Err(NbisError::InvalidTemplate("invalid minutia record length".into()));
    }
    if minutiae_start > end_limit || end_limit > template_bytes.len() {
        return Err(NbisError::InvalidTemplate(
            "minutiae data exceeds template".into(),
        ));
    }

    let available = end_limit - minutiae_start;
    let max_fit = available / record_len;
    let n = num_minutiae.min(max_fit);
    // Truncate when templates advertise more minutiae than the block can hold
    // (common with truncated gallery blobs); refuse only if nothing fits but count > 0.
    if n == 0 && num_minutiae > 0 {
        return Err(NbisError::InvalidTemplate(
            "minutiae data exceeds template".into(),
        ));
    }

    let mut minutiae = Vec::with_capacity(n);
    for i in 0..n {
        let start = minutiae_start + record_len * i;
        let mut m_bytes = [0u8; 6];
        let copy_len = record_len.min(6);
        m_bytes[..copy_len].copy_from_slice(&template_bytes[start..start + copy_len]);
        // Extended 7/8-byte records: first 6 bytes are the standard ISO minutia fields.
        minutiae.push(decode_minutia(&m_bytes, quality_scale));
    }
    Ok(minutiae)
}

fn nfiq_from_quality(finger_quality: u8) -> Nfiq2Result {
    Nfiq2Result {
        score: u32::from(finger_quality),
        actionable: Vec::new(),
        features: Vec::new(),
    }
}

fn usize_from_be_u32(bytes: &[u8]) -> Result<usize, NbisError> {
    let value = u32::from_be_bytes(bytes.try_into().map_err(|_| {
        NbisError::InvalidTemplate("invalid length field".into())
    })?) as usize;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Minutia, MinutiaKind, Nfiq2Result};

    #[test]
    fn iso_2011_header_layout() {
        let m = Minutia {
            x: 10,
            y: 20,
            direction: 4,
            reliability: 0.8,
            kind: MinutiaKind::RidgeEnding,
        };
        let minutiae = Minutiae::new(
            vec![m],
            100,
            100,
            Nfiq2Result {
                score: 75,
                actionable: Vec::new(),
                features: Vec::new(),
            },
            None,
        );
        let buf = to_iso_19794_2_2011(&minutiae).unwrap();
        assert_eq!(&buf[0..4], ISO_2011_MAGIC);
        assert_eq!(&buf[4..8], ISO_2011_VERSION);
        let reloaded = load_iso_19794_2_2011(&buf).unwrap();
        assert_eq!(reloaded.inner.len(), 1);
        assert_eq!(reloaded.quality().score, 75);
    }

    #[test]
    fn iso_2011_accepts_minbytes_8_and_trailing_padding() {
        let m = Minutia {
            x: 10,
            y: 20,
            direction: 4,
            reliability: 0.8,
            kind: MinutiaKind::RidgeEnding,
        };
        let minutiae = Minutiae::new(
            vec![m.clone(), m],
            100,
            100,
            Nfiq2Result {
                score: 50,
                actionable: Vec::new(),
                features: Vec::new(),
            },
            None,
        );
        let mut buf = to_iso_19794_2_2011(&minutiae).unwrap();
        // Locate MINBYTES byte (0x60) and rewrite to 8-byte records; expand each
        // minutia from 6 → 8 bytes and fix lengths.
        let min_fmt_idx = buf
            .iter()
            .position(|&b| b == 0x60)
            .expect("MINBYTES marker");
        let num_idx = min_fmt_idx + 1;
        let n = buf[num_idx] as usize;
        assert_eq!(n, 2);
        let minutiae_start = num_idx + 1;
        let old_minutiae = buf[minutiae_start..minutiae_start + n * 6].to_vec();
        let ext = buf[minutiae_start + n * 6..].to_vec();
        buf[min_fmt_idx] = 0x80; // MINBYTES=8
        let mut expanded = Vec::new();
        for chunk in old_minutiae.chunks(6) {
            expanded.extend_from_slice(chunk);
            expanded.extend_from_slice(&[0u8, 0u8]);
        }
        buf.truncate(minutiae_start);
        buf.extend_from_slice(&expanded);
        buf.extend_from_slice(&ext);
        // Fix total + fingerprint lengths.
        let total = buf.len() as u32;
        buf[8..12].copy_from_slice(&total.to_be_bytes());
        let fp_bytes = (total as usize - ISO_2011_HEADER_LEN) as u32;
        buf[15..19].copy_from_slice(&fp_bytes.to_be_bytes());
        buf.extend_from_slice(&[0u8; 4]); // trailing padding
        let loaded = load_iso_19794_2_2011(&buf).unwrap();
        assert_eq!(loaded.inner.len(), 2);
    }

    #[test]
    fn batch_skips_invalid_gallery_templates() {
        let mut points = Vec::new();
        for i in 0..20 {
            points.push(Minutia {
                x: 10 + i * 3,
                y: 20 + i * 2,
                direction: (i % 16) as i32,
                reliability: 0.9,
                kind: MinutiaKind::RidgeEnding,
            });
        }
        let probe = Minutiae::new(
            points,
            200,
            200,
            Nfiq2Result {
                score: 60,
                actionable: Vec::new(),
                features: Vec::new(),
            },
            None,
        );
        let probe_iso = to_iso_19794_2_2011(&probe).unwrap();
        let good = to_iso_19794_2_2011(&probe).unwrap();
        let bad = b"not-an-iso-template".to_vec();
        let scores = compare_iso_19794_2_2011_batch(&probe_iso, &[good, bad]).unwrap();
        assert_eq!(scores.len(), 2);
        assert!(scores[0] >= 0);
        assert_eq!(scores[1], 0);
        // Self-match should be strongly positive with enough minutiae.
        assert!(scores[0] > 50, "expected strong self-match, got {}", scores[0]);
    }
}
