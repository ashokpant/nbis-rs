# Build dependencies

## Required

| Dependency | Linux | macOS |
|------------|-------|-------|
| Rust **1.95.0** | `rust-toolchain.toml` | same |
| CMake, C++ toolchain | `build-essential` | Xcode CLT |
| OpenCV **4.13.x** | `./scripts/install-opencv-4.13-linux.sh` | `brew install opencv` |

```bash
./scripts/install-deps-linux.sh   # or install-deps-macos.sh
cargo build --release
```

## Python wheels

Single entry point:

```bash
make python          # host build (creates .venv)
make python-linux    # Docker (Linux wheel + bundled OpenCV)
```

Both run `scripts/build-python-wheel.sh`:

1. `maturin build` (`auditwheel = skip` in `pyproject.toml`)
2. `scripts/patch_wheel.sh` — METADATA + UniFFI stub
3. **Linux only:** `scripts/bundle_opencv_linux_wheel.sh` — vendors OpenCV 4.13, `$ORIGIN` RPATH
4. `twine check`

### Linux consumers (0.1.13+)

Wheels include `libopencv_*.so.413` next to `libnbis.so`. No host OpenCV install or `LD_LIBRARY_PATH` for NBIS.

Do not set global `LD_LIBRARY_PATH` when the app also uses `opencv-python` (`cv2`).

### macOS consumers

Wheels still link Homebrew OpenCV at runtime — `brew install opencv` (≥ 4.13).

### CI / Docker

- Linux CI: install OpenCV 4.13 + `patchelf` (see `.github/workflows/*`)
- Docker builder: `docker/nbis-rs-linux-builder.Dockerfile` + `make python-linux`

## Environment variables

| Variable | Purpose |
|----------|---------|
| `OPENCV_DIR` | `OpenCVConfig.cmake` directory |
| `OPENCV_LIB_DIR` | OpenCV libs for wheel bundling (default `/usr/local/lib`) |
| `NBIS_USE_VENDORED_OPENCV=1` | Rare: build against `ext/NFIQ2-2.3.0/opencv` sources instead of system |

## Publish to PyPI

See **[docs/PUBLISHING.md](docs/PUBLISHING.md)** for tokens, tagging, and how consuming apps should depend on `nbis-python`.

```bash
make wheels-all      # macOS + Linux wheels
make publish-check
make publish         # twine upload
```
