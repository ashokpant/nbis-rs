use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use cmake::Config;

use crate::config::{android_abi, manifest_dir, vendor_path, TargetInfo};
use crate::deps::OpenCvDeps;

pub struct Nfiq2Build {
    pub staging_root: PathBuf,
    pub include_dir: PathBuf,
    pub lib_dir: PathBuf,
}

pub fn prepare_vendor_trees(manifest: &Path) {
    let copies = [
        ("FingerJetFXOSE", "NFIQ2-2.3.0/fingerjetfxose"),
        ("digestpp", "NFIQ2-2.3.0/digestpp"),
    ];
    for (src, dst) in copies {
        let src_path = manifest.join("ext").join(src);
        let dst_path = manifest.join("ext").join(dst);
        copy_dir_recursive(&src_path, &dst_path).unwrap_or_else(|e| {
            panic!("Failed to copy ext/{src} -> ext/{dst}: {e}");
        });
    }
}

pub fn build(target: &TargetInfo, opencv: Option<&OpenCvDeps>) -> Nfiq2Build {
    let manifest = manifest_dir();
    prepare_vendor_trees(&manifest);

    let nfiq2_src = vendor_path(&manifest, &["NFIQ2-2.3.0"]);
    let mut cmake = Config::new(&nfiq2_src);

    cmake
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("EMBED_RANDOM_FOREST_PARAMETERS", "ON")
        .define("EMBEDDED_RANDOM_FOREST_PARAMETER_FCT", "3")
        .define("BUILD_NFIQ2_CLI", "OFF")
        .define("BUILD_NFIQ2_API", "OFF");

    if target.use_system_opencv {
        cmake.define("USE_SYSTEM_OPENCV", "ON");
        if let Ok(dir) = std::env::var("OPENCV_DIR") {
            cmake.define("OpenCV_DIR", dir);
        } else if let Some(ocv) = opencv {
            if let Some(cmake_path) = find_opencv_cmake_dir(ocv) {
                cmake.define("OpenCV_DIR", cmake_path.display().to_string());
            }
        }
    } else {
        cmake.define("USE_SYSTEM_OPENCV", "OFF");
        let vendored = manifest.join("ext/NFIQ2-2.3.0/opencv");
        if !vendored.join("CMakeLists.txt").exists() {
            panic!(
                "Vendored OpenCV not found at {}. \
                 Install system OpenCV 4 (recommended) or set NBIS_USE_VENDORED_OPENCV=1 \
                 and place OpenCV sources under ext/NFIQ2-2.3.0/opencv",
                vendored.display()
            );
        }
    }

    if target.is_android {
        let ndk = std::env::var("ANDROID_NDK_ROOT").expect("ANDROID_NDK_ROOT not set");
        let abi = android_abi(&target.triple).expect("Unsupported Android ABI");
        cmake.define("ANDROID_ABI", abi);
        cmake.define(
            "CMAKE_TOOLCHAIN_FILE",
            format!("{ndk}/build/cmake/android.toolchain.cmake"),
        );
    }

    let dst = cmake.build();
    let staging = dst.join("build/install_staging/nfiq2");

    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-std=c++14")
        .include(staging.join("include"))
        .include(manifest.join("src/cwrapper"))
        .file(manifest.join("src/cwrapper/nfiq_wrapper.cpp"))
        .define("NOVERBOSE", None)
        .flag_if_supported("-w")
        .compile("nfiq2_ffi");

    println!("cargo:rustc-link-lib=static=nfiq2_ffi");
    println!("cargo:rustc-link-search=native={}", staging.join("lib").display());
    println!("cargo:rustc-link-lib=static=nfiq2");

    if target.is_linux {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=z");
    } else if target.is_android {
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=android");
        println!("cargo:rustc-link-lib=c++_shared");
        if let Some(abi) = android_abi(&target.triple) {
            println!(
                "cargo:rustc-link-search=native={}",
                staging
                    .join("sdk/native/staticlibs")
                    .join(abi)
                    .display()
            );
        }
    } else if target.is_macos {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }

    Nfiq2Build {
        lib_dir: staging.join("lib"),
        include_dir: staging.join("include"),
        staging_root: staging,
    }
}

fn find_opencv_cmake_dir(ocv: &OpenCvDeps) -> Option<PathBuf> {
    for lib_path in &ocv.lib_paths {
        let candidate = lib_path.join("cmake/opencv4");
        if candidate.join("OpenCVConfig.cmake").exists() {
            return Some(candidate);
        }
    }
    None
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
