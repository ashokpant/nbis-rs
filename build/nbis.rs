use std::path::Path;

use crate::config::{manifest_dir, vendor_path, TargetInfo};
use crate::deps::{emit_opencv_rustflags, OpenCvDeps};
use crate::nfiq2::Nfiq2Build;

pub fn build_nbis_c(target: &TargetInfo, nfiq2: &Nfiq2Build, opencv: Option<&OpenCvDeps>) {
    let manifest = manifest_dir();
    build_bozorth(&manifest, target.is_windows);
    build_mindtct(&manifest, target.is_windows);
    build_sivv(&manifest, target, nfiq2, opencv);
    link_native_libraries(target, nfiq2, opencv);
}

fn build_bozorth(manifest: &Path, is_windows: bool) {
    let nbis = vendor_path(manifest, &["nbis"]);
    let mut cc = cc::Build::new();
    let bozorth = nbis.join("bozorth/src/lib/bozorth3");
    cc.file(bozorth.join("bozorth3.c"))
        .file(bozorth.join("bz_alloc.c"))
        .file(bozorth.join("bz_drvrs.c"))
        .file(bozorth.join("bz_gbls.c"))
        .file(bozorth.join("bz_io.c"))
        .file(bozorth.join("bz_sort.c"))
        .file(bozorth.join("bozorth_glue.c"))
        .include(nbis.join("commonbis/include"))
        .include(nbis.join("bozorth/include"))
        .define("NOVERBOSE", None)
        .flag_if_supported("-w");

    if is_windows {
        cc.file(manifest.join("ext/sys_time/time.cpp"))
            .include(manifest.join("ext/sys_time"));
    }
    cc.compile("bozorth");
}

fn build_mindtct(manifest: &Path, is_windows: bool) {
    let nbis = vendor_path(manifest, &["nbis"]);
    let mindtct = nbis.join("mindtct/src/lib/mindtct");
    let files = [
        "log.c", "line.c", "contour.c", "imgutil.c", "quality.c", "block.c", "loop.c",
        "mytime.c", "minutia.c", "link.c", "matchpat.c", "binar.c", "morph.c",
        "chaincod.c", "detect.c", "dft.c", "free.c", "globals.c", "init.c", "isempty.c",
        "remove.c", "ridges.c", "shape.c", "sort.c", "util.c", "maps.c", "xytreps.c",
        "getmin.c",
    ];

    let mut cc = cc::Build::new();
    for file in files {
        cc.file(mindtct.join(file));
    }
    cc.include(nbis.join("mindtct/include"))
        .define("NOVERBOSE", None)
        .flag_if_supported("-w");

    if is_windows {
        cc.file(manifest.join("ext/sys_time/time.cpp"))
            .include(manifest.join("ext/sys_time"));
    }
    cc.compile("mindtct");
}

fn build_sivv(
    manifest: &Path,
    target: &TargetInfo,
    nfiq2: &Nfiq2Build,
    opencv: Option<&OpenCvDeps>,
) {
    let nbis = vendor_path(manifest, &["nbis"]);
    let mut cc = cc::Build::new();
    cc.cpp(true)
        .flag_if_supported("-std=c++11")
        .file(nbis.join("misc/sivv/src/SIVVCore.cpp"))
        .file(nbis.join("misc/sivv/src/sivv_wrapper.cpp"))
        .include(nbis.join("misc/sivv/include"))
        .include(&nfiq2.include_dir)
        .define("NOVERBOSE", None)
        .flag_if_supported("-w");

    if target.use_system_opencv {
        if let Some(ocv) = opencv {
            for inc in &ocv.include_paths {
                cc.include(inc);
                let opencv4 = inc.join("opencv4");
                if opencv4.exists() {
                    cc.include(opencv4);
                }
            }
        }
    } else {
        cc.include(nfiq2.include_dir.join("opencv4"));
    }

    if target.is_windows {
        cc.file(manifest.join("ext/sys_time/time.cpp"))
            .include(manifest.join("ext/sys_time"));
    }
    if target.is_android {
        cc.include(
            nfiq2
                .staging_root
                .parent()
                .unwrap()
                .join("sdk/native/jni/include"),
        );
    }

    cc.compile("sivv");
}

fn link_native_libraries(target: &TargetInfo, nfiq2: &Nfiq2Build, opencv: Option<&OpenCvDeps>) {
    if target.is_android {
        if let Some(abi) = crate::config::android_abi(&target.triple) {
            println!(
                "cargo:rustc-link-search=native={}",
                nfiq2.lib_dir.join(abi).display()
            );
        }
    } else if !target.is_macos {
        println!(
            "cargo:rustc-link-search=native={}",
            nfiq2.lib_dir.display()
        );
    }

    println!("cargo:rustc-link-lib=static=FRFXLL_static");

    if target.use_system_opencv {
        if let Some(ocv) = opencv {
            emit_opencv_rustflags(ocv);
        }
    } else if target.is_windows {
        link_windows_vendored_opencv(nfiq2);
    } else {
        for lib in ["opencv_imgproc", "opencv_ml", "opencv_imgcodecs", "opencv_core"] {
            println!("cargo:rustc-link-lib=static={lib}");
        }
    }

    println!("cargo:rustc-link-lib=z");
    println!("cargo:rerun-if-env-changed=OPENCV_DIR");
    println!("cargo:rerun-if-env-changed=NBIS_USE_VENDORED_OPENCV");
}

fn link_windows_vendored_opencv(nfiq2: &Nfiq2Build) {
    let lib_root = nfiq2.staging_root.display();
    println!("cargo:rustc-link-search=native=C:/msys64/mingw64/lib");
    println!("cargo:rustc-link-search=native={lib_root}");
    for lib in [
        "opencv_imgproc4100",
        "opencv_ml4100",
        "opencv_imgcodecs4100",
        "opencv_core4100",
        "openblas",
        "gomp",
        "stdc++",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
}
