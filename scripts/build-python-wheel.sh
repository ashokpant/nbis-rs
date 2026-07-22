#!/usr/bin/env bash
# Build nbis-python wheel: maturin → patch METADATA/stub → bundle OpenCV (Linux) → twine check.
#
# Env:
#   NBIS_DIST_DIR          output directory (default: dist)
#   NBIS_CARGO_TARGET_DIR  optional cargo target dir (Docker: target/linux-docker)
#   NBIS_SKIP_HOST_DEPS    set to 1 to skip install-deps-* (Docker / CI)
#   NBIS_SKIP_STUB_SYNC    set to 1 to skip UniFFI stub sync
#   NBIS_SKIP_VERIFY       set to 1 to skip Linux import smoke test
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DIST_DIR="${NBIS_DIST_DIR:-dist}"
CARGO_TARGET_DIR="${NBIS_CARGO_TARGET_DIR:-}"
export PATH="${NBIS_PATH:-${PATH}}"

mkdir -p "$DIST_DIR"

if [ "${NBIS_SKIP_HOST_DEPS:-0}" != "1" ]; then
  case "$(uname -s)" in
    Linux)
      if [ -f /etc/os-release ] && grep -qiE 'ubuntu|debian' /etc/os-release; then
        ./scripts/install-deps-linux.sh
      fi
      ;;
    Darwin)
      ./scripts/install-deps-macos.sh
      if [ -z "${OPENCV_DIR:-}" ]; then
        if [ -f "${HOME}/.local/opencv-4.13.0/lib/cmake/opencv4/OpenCVConfig.cmake" ]; then
          OPENCV_DIR="${HOME}/.local/opencv-4.13.0/lib/cmake/opencv4"
        elif [ -f /usr/local/lib/cmake/opencv4/OpenCVConfig.cmake ]; then
          OPENCV_DIR=/usr/local/lib/cmake/opencv4
        elif [ -f /opt/homebrew/opt/opencv/lib/cmake/opencv4/OpenCVConfig.cmake ]; then
          OPENCV_DIR=/opt/homebrew/opt/opencv/lib/cmake/opencv4
        elif [ -f /usr/local/opt/opencv/lib/cmake/opencv4/OpenCVConfig.cmake ]; then
          OPENCV_DIR=/usr/local/opt/opencv/lib/cmake/opencv4
        fi
      fi
      export OPENCV_DIR
      if [ -n "${OPENCV_DIR:-}" ]; then
        ocv_prefix="$(cd "${OPENCV_DIR}/../../.." && pwd)"
        export DYLD_FALLBACK_LIBRARY_PATH="${ocv_prefix}/lib:${DYLD_FALLBACK_LIBRARY_PATH:-}"
        export PKG_CONFIG_PATH="${ocv_prefix}/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
      fi
      ;;
  esac
fi

if [ -n "$CARGO_TARGET_DIR" ]; then
  export CARGO_TARGET_DIR
  mkdir -p "$CARGO_TARGET_DIR"
else
  unset CARGO_TARGET_DIR
fi

# Use project venv when present (local dev); Docker/CI set NBIS_PATH to venv bin.
if [ -z "${NBIS_PATH:-}" ] && [ -x "$ROOT/.venv/bin/maturin" ]; then
  export PATH="$ROOT/.venv/bin:$PATH"
fi

if ! command -v maturin >/dev/null; then
  echo "maturin not found; install with: pip install 'maturin>=1.5,<2.0'" >&2
  exit 1
fi

maturin build \
  --release \
  --manifest-path Cargo.toml \
  --out "$DIST_DIR" \
  --strip \
  --locked

wheel="$(ls -t "$DIST_DIR"/nbis_python*.whl | head -n 1)"
MATURIN_DIST_DIR="$DIST_DIR" WHEEL_GLOB='nbis_python*.whl' bash "$ROOT/scripts/patch_wheel.sh"

wheel="$(ls -t "$DIST_DIR"/nbis_python*.whl | head -n 1)"

if [ "$(uname -s)" = "Linux" ]; then
  bash "$ROOT/scripts/bundle_opencv_linux_wheel.sh" "$wheel" "$DIST_DIR"
  wheel="$(ls -t "$DIST_DIR"/nbis_python*.whl | head -n 1)"
  if [ "${NBIS_SKIP_VERIFY:-0}" != "1" ] && [ -f test_data/p1/p1_1.png ]; then
    bash "$ROOT/scripts/verify_bundled_linux_wheel.sh" "$wheel"
  fi
fi

if [ "${NBIS_SKIP_STUB_SYNC:-0}" != "1" ]; then
  case "$(uname -s)" in
    Darwin) bash "$ROOT/scripts/sync_uniffi_stub.sh" ;;
    *) bash "$ROOT/scripts/sync_uniffi_stub.sh" --if-present ;;
  esac
fi

command -v twine >/dev/null && twine check "$wheel" || true

echo "Built: $wheel"
