use std::path::{Path, PathBuf};

/// Minimal vendored sources (NBIS subset, NFIQ2, FingerJet, digestpp).
pub const VENDOR: &str = "ext";

pub struct TargetInfo {
    pub triple: String,
    pub is_windows: bool,
    pub is_android: bool,
    pub is_linux: bool,
    pub is_macos: bool,
    /// Use system OpenCV (OPENCV_DIR / pkg-config). Off only with NBIS_USE_VENDORED_OPENCV=1.
    pub use_system_opencv: bool,
}

impl TargetInfo {
    pub fn detect() -> Self {
        let triple = std::env::var("TARGET").unwrap_or_default();
        let is_windows = triple.contains("windows");
        let is_android = triple.contains("android");
        let is_linux = triple.contains("linux") && !is_android;
        let is_macos = triple.contains("apple") || triple.contains("darwin");

        let use_system_opencv = std::env::var("NBIS_USE_VENDORED_OPENCV").is_err();

        Self {
            triple,
            is_windows,
            is_android,
            is_linux,
            is_macos,
            use_system_opencv,
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
}

pub fn vendor_path(manifest: &Path, parts: &[&str]) -> PathBuf {
    parts.iter().fold(manifest.join(VENDOR), |p, s| p.join(s))
}

pub fn android_abi(triple: &str) -> Option<&'static str> {
    match triple {
        t if t.contains("aarch64") => Some("arm64-v8a"),
        t if t.contains("armv7") => Some("armeabi-v7a"),
        t if t.contains("x86_64") => Some("x86_64"),
        t if t.contains("i686") => Some("x86"),
        _ => None,
    }
}
