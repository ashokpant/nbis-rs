# Build dependencies

## Required

| Dependency | Linux (Debian/Ubuntu) | macOS (Homebrew) |
|------------|------------------------|------------------|
| Rust **1.95.0** (edition **2024**) | [rustup](https://rustup.rs) + `rust-toolchain.toml` | same |
| CMake ≥ 3.16 | `cmake` | `brew install cmake` |
| C/C++ toolchain | `build-essential` | Xcode CLT |
| OpenCV **4.13.x** | `./scripts/install-opencv-4.13-linux.sh` | `brew install opencv` (≥ 4.13) |
| pkg-config | `pkg-config` | `brew install pkg-config` |

```bash
./scripts/install-deps-linux.sh   # Linux (builds OpenCV 4.13.0 to /usr/local)
./scripts/install-deps-macos.sh   # macOS
cargo build --release
```

## Environment variables

| Variable | Purpose |
|----------|---------|
| `OPENCV_DIR` | Path to `OpenCVConfig.cmake` (default Linux: `/usr/local/lib/cmake/opencv4`) |
| `OPENCV_INCLUDE_DIR` | Override OpenCV headers |
| `NBIS_USE_VENDORED_OPENCV=1` | Use sources in `ext/NFIQ2-2.3.0/opencv` (fallback only) |

## Python (3.10+)

```bash
make python          # wheel for your current OS (macOS or Linux)
make python-linux    # Linux x86_64 wheel via Docker (from macOS or anywhere with Docker)
```

Linux cross-build uses Docker image `nbis-rs-linux-builder:24.04` (Rust **1.95.0**, OpenCV **4.13.0**, maturin preinstalled). First run builds the base image automatically; rebuild after toolchain changes with `make linux-baseimage`. Wheels are built in `dist/linux/`, then copied to `dist/`.

```bash
make linux-baseimage   # optional: rebuild base image
make python-linux      # uses base image + cargo cache volumes
```

`LINUX_PLATFORM=linux/arm64 make linux-baseimage` then `make python-linux` for ARM64 Linux wheels.

Wheels use `auditwheel = "skip"` (no bundled OpenCV) and `compatibility = "manylinux_2_28"` on Linux so PyPI accepts the wheel tag. At runtime the host must provide **OpenCV 4.13** shared libraries (`libopencv_core.so.413`, etc.).

### Docker / container runtime (Linux)

Published Linux wheels link against **OpenCV 4.13** (`libopencv_core.so.413`, etc.). `pip install opencv-python` does **not** satisfy this — you need the same C++ OpenCV your wheel was built with.

**Option A — install into `/usr/local` (matches our builder):**

```dockerfile
COPY scripts/install-opencv-4.13-linux.sh /tmp/
RUN chmod +x /tmp/install-opencv-4.13-linux.sh && /tmp/install-opencv-4.13-linux.sh
ENV LD_LIBRARY_PATH=/usr/local/lib:${LD_LIBRARY_PATH}
```

**Option B — copy libs from a builder stage** that already ran `install-opencv-4.13-linux.sh`.

**Option C — distro packages** only if they ship OpenCV 4.13 (most `python:3.13-slim` images today ship 4.10 and will **not** work with wheels built against 4.13).

If import fails, inspect the wheel’s native library:

```bash
python -c "import pathlib, nbis.nbis.nbis as m; print(pathlib.Path(m.__file__).with_name('libnbis.so'))"
ldd "$(python -c "import pathlib, nbis.nbis.nbis as m; print(pathlib.Path(m.__file__).with_name('libnbis.so'))")"
```

Linux wheels use a stub at `bindings/python/nbis-python/_uniffi_stubs/nbis.py`. **`make python` on macOS** updates it automatically after each wheel build; commit the stub when UniFFI exports change. `make build` / `make build-release` also sync if `target/maturin/.../nbis.py` already exists.

### Publish to PyPI

```bash
make python          # and/or make python-linux
make publish         # twine upload (uses .venv from make python)
```

**GitHub Actions:** [`ci.yml`](.github/workflows/ci.yml) (CI), [`release.yml`](.github/workflows/release.yml)

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
