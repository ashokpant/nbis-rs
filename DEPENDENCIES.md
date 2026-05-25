# Build dependencies

## Required

| Dependency | Linux (Debian/Ubuntu) | macOS (Homebrew) |
|------------|------------------------|------------------|
| Rust ≥ 1.80 | [rustup](https://rustup.rs) | rustup |
| CMake ≥ 3.16 | `cmake` | `brew install cmake` |
| C/C++ toolchain | `build-essential` | Xcode CLT |
| OpenCV 4.x | `libopencv-dev` | `brew install opencv` |
| pkg-config | `pkg-config` | `brew install pkg-config` |

```bash
./scripts/install-deps-linux.sh   # Linux
./scripts/install-deps-macos.sh   # macOS
cargo build --release
```

## Environment variables

| Variable | Purpose |
|----------|---------|
| `OPENCV_DIR` | Path to `OpenCVConfig.cmake` (e.g. `/opt/homebrew/opt/opencv/lib/cmake/opencv4`) |
| `OPENCV_INCLUDE_DIR` | Override OpenCV headers |
| `NBIS_USE_VENDORED_OPENCV=1` | Use sources in `ext/NFIQ2-2.3.0/opencv` (fallback only) |

## Python (3.10+)

```bash
make python
```

Wheels are built with `auditwheel = "skip"` so maturin does not bundle Homebrew dylibs (avoids OpenCV/OpenEXR version skew on macOS). At runtime you need system OpenCV 4 installed.

## macOS troubleshooting

If `maturin` fails repairing the wheel (`libOpenEXR` / `libIlmThread` not found), either:

```bash
brew reinstall openexr opencv
make python
```

or build with repair disabled (already the default in `pyproject.toml`):

```bash
maturin build --release --auditwheel=skip
```

If tests fail loading OpenEXR at runtime:

```bash
brew reinstall openexr opencv
```
