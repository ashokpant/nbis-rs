use std::path::PathBuf;

use pkg_config::Config;

/// Resolved OpenCV 4 installation for compile and link.
pub struct OpenCvDeps {
    pub include_paths: Vec<PathBuf>,
    pub lib_paths: Vec<PathBuf>,
}

pub fn find_opencv() -> OpenCvDeps {
    if let Ok(dir) = std::env::var("OPENCV_DIR") {
        return find_opencv_from_cmake_dir(&dir);
    }

    if let Some(dir) = default_opencv_cmake_dir() {
        return find_opencv_from_cmake_dir(dir.to_str().unwrap());
    }

    let config = Config::new()
        .cargo_metadata(false)
        .atleast_version("4.13")
        .probe("opencv4")
        .or_else(|_| {
            Config::new()
                .cargo_metadata(false)
                .atleast_version("4.13")
                .probe("opencv5")
        })
        .or_else(|_| {
            Config::new()
                .cargo_metadata(false)
                .atleast_version("4.13")
                .probe("opencv")
        })
        .unwrap_or_else(|e| {
            panic!(
                "OpenCV 4.13+ is required. On Linux run ./scripts/install-deps-linux.sh \
                 (builds 4.13.0 to /usr/local). On macOS: brew install opencv. \
                 Or set OPENCV_DIR to the directory containing OpenCVConfig.cmake. \
                 Error: {e}"
            );
        });

    OpenCvDeps {
        include_paths: config.include_paths,
        lib_paths: config.link_paths,
    }
}

fn find_opencv_from_cmake_dir(dir: &str) -> OpenCvDeps {
    let cmake_dir = PathBuf::from(dir);

    let include = if let Ok(path) = std::env::var("OPENCV_INCLUDE_DIR") {
        PathBuf::from(path)
    } else {
        resolve_opencv_include_dir(&cmake_dir).unwrap_or_else(|| {
            panic!(
                "Could not infer OpenCV include path from OPENCV_DIR={dir}. \
                 Set OPENCV_INCLUDE_DIR explicitly."
            )
        })
    };

    let lib_dir = resolve_opencv_lib_dir(&cmake_dir).unwrap_or_else(|| PathBuf::from("/usr/lib"));

    OpenCvDeps {
        include_paths: vec![include],
        lib_paths: vec![lib_dir],
    }
}

fn resolve_opencv_include_dir(cmake_dir: &PathBuf) -> Option<PathBuf> {
    let cmake_str = cmake_dir.to_string_lossy();

    // Debian/Ubuntu: /usr/lib/<triplet>/cmake/opencv4 -> /usr/include/opencv4
    if cmake_str.starts_with("/usr/lib/")
        && (cmake_str.ends_with("/cmake/opencv4") || cmake_str.ends_with("/cmake/opencv5"))
    {
        for name in ["opencv4", "opencv5"] {
            let include = PathBuf::from(format!("/usr/include/{name}"));
            if include.exists() {
                return Some(include);
            }
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if cmake_dir.ends_with("opencv4") || cmake_dir.ends_with("opencv5") {
        if let Some(root) = cmake_dir
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            candidates.push(root.join("include/opencv4"));
            candidates.push(root.join("include/opencv5"));
            candidates.push(root.join("include"));
        }
    }
    candidates.push(cmake_dir.join("include/opencv4"));
    candidates.push(cmake_dir.join("include/opencv5"));
    candidates.push(cmake_dir.join("include"));

    candidates.into_iter().find(|p| p.exists())
}

fn resolve_opencv_lib_dir(cmake_dir: &PathBuf) -> Option<PathBuf> {
    let cmake_str = cmake_dir.to_string_lossy();

    // Debian/Ubuntu: libs live next to cmake/, not under a top-level lib/ subtree.
    if cmake_str.starts_with("/usr/lib/")
        && (cmake_str.ends_with("/cmake/opencv4") || cmake_str.ends_with("/cmake/opencv5"))
    {
        let lib_dir = cmake_dir.parent()?.parent()?;
        if lib_dir.exists() {
            return Some(lib_dir.to_path_buf());
        }
    }

    // Homebrew: .../opt/opencv/lib/cmake/opencv{4,5} -> .../opt/opencv/lib
    if cmake_dir.ends_with("opencv4") || cmake_dir.ends_with("opencv5") {
        if let Some(lib_dir) = cmake_dir
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|p| p.join("lib"))
            .filter(|p| p.exists())
        {
            return Some(lib_dir);
        }
    }

    None
}

fn default_opencv_cmake_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut candidates = Vec::new();
    if let Some(home) = home {
        candidates.push(home.join(".local/opencv-4.13.0/lib/cmake/opencv4"));
    }
    candidates.extend(
        [
            "/usr/local/lib/cmake/opencv4",
            "/opt/homebrew/opt/opencv/lib/cmake/opencv4",
            "/usr/local/opt/opencv/lib/cmake/opencv4",
            "/usr/lib/aarch64-linux-gnu/cmake/opencv4",
            "/usr/lib/x86_64-linux-gnu/cmake/opencv4",
        ]
        .iter()
        .map(PathBuf::from),
    );
    candidates
        .into_iter()
        .find(|p| p.join("OpenCVConfig.cmake").exists())
}

const OPENCV_COMPONENTS: &[&str] = &[
    "opencv_core",
    "opencv_imgproc",
    "opencv_ml",
    "opencv_imgcodecs",
];

fn emit_macos_rpaths() {
    let prefix = std::env::var("HOMEBREW_PREFIX").unwrap_or_else(|_| "/opt/homebrew".into());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{prefix}/lib");
    if let Ok(home) = std::env::var("HOME") {
        let local = PathBuf::from(home).join(".local/opencv-4.13.0/lib");
        if local.exists() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", local.display());
        }
    }
    if let Ok(ocv) = std::env::var("OPENCV_DIR") {
        if let Some(lib) = PathBuf::from(ocv)
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
        {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib.display());
        }
    }
}

pub fn emit_opencv_rustflags(deps: &OpenCvDeps) {
    let target = std::env::var("TARGET").unwrap_or_default();

    for path in &deps.lib_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }

    for component in OPENCV_COMPONENTS {
        println!("cargo:rustc-link-lib=dylib={component}");
    }

    if target.contains("apple") {
        emit_macos_rpaths();
    }

    for path in &deps.include_paths {
        println!("cargo:include={}", path.display());
    }
}
