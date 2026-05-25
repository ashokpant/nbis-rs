#!/usr/bin/env bash
# Build the nbis-py wheel. Uses system OpenCV; skips maturin dylib repair on macOS.
set -euo pipefail

mkdir -p dist

case "$(uname -s)" in
  Linux)
    if [ -f /etc/os-release ] && grep -qiE 'ubuntu|debian' /etc/os-release; then
      ./scripts/install-deps-linux.sh
    fi
    ;;
  Darwin)
    ./scripts/install-deps-macos.sh
    # shellcheck disable=SC1091
    [ -f /opt/homebrew/opt/opencv/lib/cmake/opencv4/OpenCVConfig.cmake ] \
      && export OPENCV_DIR="${OPENCV_DIR:-/opt/homebrew/opt/opencv/lib/cmake/opencv4}"
    [ -f /usr/local/opt/opencv/lib/cmake/opencv4/OpenCVConfig.cmake ] \
      && export OPENCV_DIR="${OPENCV_DIR:-/usr/local/opt/opencv/lib/cmake/opencv4}"
    ;;
esac

# Pick the newest available Python >= 3.10 for the build venv.
PYTHON=""
for candidate in python3.12 python3.11 python3.10 python3; do
  if command -v "$candidate" >/dev/null 2>&1; then
    version=$("$candidate" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
    major=${version%%.*}
    minor=${version#*.}
    if [ "$major" -eq 3 ] && [ "$minor" -ge 10 ]; then
      PYTHON=$candidate
      break
    fi
  fi
done
if [ -z "$PYTHON" ]; then
  echo "Python 3.10+ is required. Install python3.10, python3.11, or python3.12."
  exit 1
fi
echo "Using $PYTHON ($($PYTHON --version))"

VENV_DIR=".venv"
if [ ! -d "$VENV_DIR" ]; then
  "$PYTHON" -m venv "$VENV_DIR"
fi
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"

pip install --upgrade pip
pip install "maturin>=1.5,<2.0" twine

# auditwheel=skip: do not run delocate repair (Homebrew OpenCV/OpenEXR version skew breaks repair).
maturin build \
  --release \
  --manifest-path Cargo.toml \
  --out dist \
  --strip \
  --locked \
  --auditwheel=skip

bash ./patch_maturin_wheel.sh
twine check dist/nbis_py*.whl

echo "Built: dist/nbis_py-*.whl"
echo "Install with the same Python (>=3.10):"
echo "  $VENV_DIR/bin/pip install dist/nbis_py-*.whl"
echo "  # or: $PYTHON -m pip install dist/nbis_py-*.whl"
