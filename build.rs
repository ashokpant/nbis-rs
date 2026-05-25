#[path = "build/config.rs"]
mod config;
#[path = "build/deps.rs"]
mod deps;
#[path = "build/nbis.rs"]
mod nbis;
#[path = "build/nfiq2.rs"]
mod nfiq2;

use config::TargetInfo;

fn main() {
    println!("cargo:rerun-if-env-changed=CLIPPY");
    println!("cargo:rerun-if-changed=ext/NFIQ2-2.3.0/CMakeLists.txt");
    println!("cargo:rerun-if-changed=ext/NFIQ2-2.3.0/NFIQ2/NFIQ2Algorithm/CMakeLists.txt");

    let target = TargetInfo::detect();
    let opencv = if target.use_system_opencv {
        Some(deps::find_opencv())
    } else {
        None
    };

    let nfiq2 = nfiq2::build(&target, opencv.as_ref());
    nbis::build_nbis_c(&target, &nfiq2, opencv.as_ref());
}
