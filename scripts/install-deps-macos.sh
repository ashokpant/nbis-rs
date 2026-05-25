#!/usr/bin/env bash
# System dependencies for building nbis-rs on macOS (Homebrew).
set -euo pipefail

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required: https://brew.sh"
  exit 1
fi

echo "Installing build dependencies..."
brew install cmake pkg-config openexr opencv

# Reinstall only when OpenCV references OpenEXR dylibs that are missing (common after brew upgrades).
needs_reinstall=false
if [ -f /opt/homebrew/opt/opencv/lib/libopencv_imgcodecs.dylib ]; then
  while IFS= read -r dep; do
    if [[ "$dep" == /opt/homebrew/opt/openexr/lib/* ]] && [[ ! -f "$dep" ]]; then
      needs_reinstall=true
      break
    fi
  done < <(otool -L /opt/homebrew/opt/opencv/lib/libopencv_imgcodecs.dylib 2>/dev/null | awk 'NR>1 {print $1}')
fi
if $needs_reinstall; then
  echo "OpenCV/OpenEXR mismatch detected; reinstalling..."
  brew reinstall openexr opencv
fi

if [ -d "/opt/homebrew/opt/opencv/lib/cmake/opencv4" ]; then
  export OPENCV_DIR="/opt/homebrew/opt/opencv/lib/cmake/opencv4"
elif [ -d "/usr/local/opt/opencv/lib/cmake/opencv4" ]; then
  export OPENCV_DIR="/usr/local/opt/opencv/lib/cmake/opencv4"
fi

echo "Done."
echo "  OPENCV_DIR=${OPENCV_DIR:-<set manually if needed>}"
echo "  cargo build --release"
echo "  make python   # uses maturin --auditwheel=skip"
