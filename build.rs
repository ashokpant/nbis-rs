use std::{env, fs, io, path::{Path, PathBuf}};

fn android_abi_from_target(target: &str) -> Option<&'static str> {
    match target {
        t if t.contains("aarch64") => Some("arm64-v8a"),
        t if t.contains("armv7") => Some("armeabi-v7a"),
        t if t.contains("x86_64") => Some("x86_64"),
        t if t.contains("i686") => Some("x86"),
        _ => None,
    }
}

fn copy_nfiq2_dirs() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);
    
    let dirs = vec![
        ("ext/opencv-4.10.0", "ext/NFIQ2-2.3.0/opencv"),
        ("ext/FingerJetFXOSE", "ext/NFIQ2-2.3.0/fingerjetfxose"),
        ("ext/digestpp", "ext/NFIQ2-2.3.0/digestpp"),
        ("ext/libbiomeval-10.0", "ext/NFIQ2-2.3.0/libbiomeval"),
    ];
    
    for (src, dst) in dirs {
        let src_path = manifest_path.join(src);
        let dst_path = manifest_path.join(dst);
        copy_dir_recursive(&src_path, &dst_path)
            .expect(&format!("Failed to copy {} to {}", src, dst));
    }
}

fn build_nfiq2() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);
    
    let target = env::var("TARGET").unwrap_or_default();
    let is_android = target.contains("android");
    let is_linux = target.contains("linux") && !target.contains("android");
    let is_macos = target.contains("apple") || target.contains("darwin");
    
    copy_nfiq2_dirs();

    let nfiq2_src = manifest_path.join("ext/NFIQ2-2.3.0");
    let mut cmake = cmake::Config::new(&nfiq2_src);
    cmake
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("CMAKE_INSTALL_PREFIX", "NFIQ2-2.3.0/install")
        .define("EMBED_RANDOM_FOREST_PARAMETERS", "ON")
        .define("EMBEDDED_RANDOM_FOREST_PARAMETER_FCT", "3")
        .define("BUILD_NFIQ2_CLI", "OFF");

    if is_android {
        let ndk = env::var("ANDROID_NDK_ROOT").expect("ANDROID_NDK_ROOT not set");
        let abi = android_abi_from_target(&target).expect("Unsupported Android ABI");
        cmake.define("ANDROID_ABI", abi);
        cmake.define("CMAKE_TOOLCHAIN_FILE", format!("{ndk}/build/cmake/android.toolchain.cmake"));
    }

    let dst = cmake.build();

    let nfiq2_include_path = dst.join("build/install_staging/nfiq2/include");
    let nfiq2_lib_path = dst.join("build/install_staging/nfiq2/lib");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let opencv_lib_path = out_dir.join("build/install_staging/nfiq2/lib");

    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-std=c++14")
        .include(&nfiq2_include_path)
        .include("src/cwrapper")
        .file("src/cwrapper/nfiq_wrapper.cpp")
        .define("NOVERBOSE", None)
        .flag_if_supported("-w")
        .compile("nfiq2_ffi");

    println!("cargo:rustc-link-lib=static=nfiq2_ffi");
    println!("cargo:rustc-link-search=native={}", nfiq2_lib_path.display());
    println!("cargo:rustc-link-search=native={}", opencv_lib_path.display());

    if is_android {
        let abi = android_abi_from_target(&target).expect("Unsupported ABI");
        println!("cargo:rustc-link-search=native={}", 
            dst.join(format!("build/install_staging/nfiq2/sdk/native/staticlibs/{abi}")).display());
    }

    println!("cargo:rustc-link-lib=static=nfiq2");

    if is_linux {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=z");
    } else if is_android {
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=android");
        println!("cargo:rustc-link-lib=c++_shared");
    } else if is_macos {
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=framework=OpenCL");
    }

    dst
}

fn build_bozorth(is_windows: bool) {
    let mut cc = cc::Build::new();
    cc.file("ext/nbis/bozorth/src/lib/bozorth3/bozorth3.c")
        .file("ext/nbis/bozorth/src/lib/bozorth3/bz_alloc.c")
        .file("ext/nbis/bozorth/src/lib/bozorth3/bz_drvrs.c")
        .file("ext/nbis/bozorth/src/lib/bozorth3/bz_gbls.c")
        .file("ext/nbis/bozorth/src/lib/bozorth3/bz_io.c")
        .file("ext/nbis/bozorth/src/lib/bozorth3/bz_sort.c")
        .file("ext/nbis/bozorth/src/lib/bozorth3/bozorth_glue.c")
        .include("ext/nbis/commonbis/include")
        .include("ext/nbis/bozorth/include")
        .define("NOVERBOSE", None)
        .flag_if_supported("-w");

    if is_windows {
        cc.file("ext/sys_time/time.cpp").include("ext/sys_time");
    }

    cc.compile("bozorth");
}

fn build_mindtct(is_windows: bool) {
    let mut cc = cc::Build::new();
    let files = vec![
        "log.c", "line.c", "contour.c", "imgutil.c", "quality.c",
        "block.c", "loop.c", "mytime.c", "minutia.c", "link.c",
        "matchpat.c", "binar.c", "morph.c", "chaincod.c", "detect.c",
        "dft.c", "free.c", "globals.c", "init.c", "isempty.c",
        "remove.c", "ridges.c", "shape.c", "sort.c", "util.c",
        "maps.c", "xytreps.c", "getmin.c",
    ];
    
    for file in files {
        cc.file(format!("ext/nbis/mindtct/src/lib/mindtct/{}", file));
    }

    cc.include("ext/nbis/mindtct/include")
        .define("NOVERBOSE", None)
        .flag_if_supported("-w");

    if is_windows {
        cc.file("ext/sys_time/time.cpp").include("ext/sys_time");
    }

    cc.compile("mindtct");
}

fn build_sivv(dst: &PathBuf, target: &str) {
    let is_android = target.contains("android");
    let is_windows = target.contains("windows");
    
    let mut cc = cc::Build::new();
    cc.cpp(true)
        .flag("-std=c++11")
        .file("ext/nbis/misc/sivv/src/SIVVCore.cpp")
        .file("ext/nbis/misc/sivv/src/sivv_wrapper.cpp")
        .include("ext/nbis/misc/sivv/include")
        .include(dst.join("build/install_staging/nfiq2/include"))
        .include(dst.join("build/install_staging/nfiq2/include/opencv4"))
        .define("NOVERBOSE", None)
        .flag_if_supported("-w");

    if is_windows {
        cc.file("ext/sys_time/time.cpp").include("ext/sys_time");
    }

    if is_android {
        cc.include(dst.join("build/install_staging/nfiq2/sdk/native/jni/include"));
    }

    cc.compile("sivv");
}

fn link_libraries(dst: &PathBuf, target: &str) {
    let is_android = target.contains("android");
    let is_linux = target.contains("linux") && !target.contains("android");
    let is_windows = target.contains("windows");

    if is_android || is_linux {
        let lib_dir = if is_android {
            let abi = android_abi_from_target(&target).expect("Unsupported target");
            dst.join("build/install_staging/nfiq2/lib").join(abi)
        } else {
            dst.join("build/install_staging/nfiq2/lib")
        };
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
    } else {
        println!("cargo:rustc-link-search=native={}/lib", dst.display());
    }

    if !is_windows {
        println!("cargo:rustc-link-lib=static=opencv_imgproc");
        println!("cargo:rustc-link-lib=static=opencv_ml");
        println!("cargo:rustc-link-lib=static=opencv_imgcodecs");
        println!("cargo:rustc-link-lib=static=opencv_core");
        println!("cargo:rustc-link-lib=static=FRFXLL_static");
    } else {
        let lib_src_str = format!("{}/build/install_staging/nfiq2", dst.display());
        let lib_src = Path::new(&lib_src_str);
        let lib_dst = Path::new("ext/nfiq2_libs");
        copy_dir_recursive(lib_src, lib_dst).expect("Failed to copy libraries");

        println!("cargo:rustc-link-search=native=C:/msys64/mingw64/lib");
        println!("cargo:rustc-link-search=native={}/lib", lib_src_str);
        println!("cargo:rustc-link-search=native={}/x64/mingw/staticlib", lib_src_str);
        println!("cargo:rustc-link-lib=static=opencv_imgproc4100");
        println!("cargo:rustc-link-lib=static=opencv_ml4100");
        println!("cargo:rustc-link-lib=static=opencv_imgcodecs4100");
        println!("cargo:rustc-link-lib=static=opencv_core4100");
        println!("cargo:rustc-link-lib=static=FRFXLL_static");
        println!("cargo:rustc-link-lib=static=openblas");
        println!("cargo:rustc-link-lib=static=gomp");
        println!("cargo:rustc-link-lib=static=stdc++");
    }

    println!("cargo:rustc-link-lib=z");
    println!("cargo:rerun-if-changed=ext/nbis/bozorth/src/lib/bozorth3/bozorth3.c");
    println!("cargo:rerun-if-changed=ext/nbis/bozorth/include");
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

fn main() {
    println!("cargo:rerun-if-env-changed=CLIPPY");

    let target = env::var("TARGET").unwrap_or_default();
    let is_windows = target.contains("windows");

    let dst = build_nfiq2();
    build_bozorth(is_windows);
    build_mindtct(is_windows);
    build_sivv(&dst, &target);
    link_libraries(&dst, &target);
}
