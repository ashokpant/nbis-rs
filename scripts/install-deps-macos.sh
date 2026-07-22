#!/usr/bin/env bash
# System dependencies for building nbis-rs on macOS (Homebrew).
set -euo pipefail

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required: https://brew.sh"
  exit 1
fi

opencv_usable() {
  # Prefer a real OpenCV 4 install (NFIQ2 requires find_package(OpenCV 4)).
  [ -f "${HOME}/.local/opencv-4.13.0/lib/cmake/opencv4/OpenCVConfig.cmake" ] \
    || [ -f /usr/local/lib/cmake/opencv4/OpenCVConfig.cmake ] \
    || [ -f /opt/homebrew/opt/opencv/lib/cmake/opencv4/OpenCVConfig.cmake ] \
    || [ -f /usr/local/opt/opencv/lib/cmake/opencv4/OpenCVConfig.cmake ]
}

brew_opencv_is_v5() {
  [ -f /opt/homebrew/opt/opencv/lib/cmake/opencv5/OpenCVConfig.cmake ] \
    || [ -f /usr/local/opt/opencv/lib/cmake/opencv5/OpenCVConfig.cmake ]
}

echo "Installing build dependencies..."
# cmake / pkg-config / openexr are usually already present; do not fail the whole
# build when Homebrew cannot symlink Qt (qt vs qtshadertools conflicts are common).
if ! brew list cmake >/dev/null 2>&1; then
  brew install cmake || true
fi
if ! brew list pkg-config >/dev/null 2>&1 && ! brew list pkgconf >/dev/null 2>&1; then
  brew install pkg-config || brew install pkgconf || true
fi
if ! brew list openexr >/dev/null 2>&1; then
  brew install openexr || true
fi

if opencv_usable; then
  echo "OpenCV 4 already present; skipping install"
elif brew_opencv_is_v5; then
  echo "Homebrew OpenCV is 5.x (NFIQ2 needs 4.x); building OpenCV 4.13 locally..."
  ./scripts/install-opencv-4.13-macos.sh
elif ! brew list opencv >/dev/null 2>&1; then
  set +e
  brew install opencv
  brew_status=$?
  set -e
  if brew_opencv_is_v5 || ! opencv_usable; then
    echo "Homebrew OpenCV is not usable for NFIQ2; building OpenCV 4.13 locally..."
    ./scripts/install-opencv-4.13-macos.sh
  elif [ "$brew_status" -ne 0 ] && ! opencv_usable; then
    echo "OpenCV install failed." >&2
    exit 1
  fi
else
  echo "No usable OpenCV 4 found; building OpenCV 4.13 locally..."
  ./scripts/install-opencv-4.13-macos.sh
fi

# Reinstall only when Homebrew OpenCV references OpenEXR dylibs that are missing.
needs_reinstall=false
if [ -f /opt/homebrew/opt/opencv/lib/libopencv_imgcodecs.dylib ] && ! brew_opencv_is_v5; then
  while IFS= read -r dep; do
    if [[ "$dep" == /opt/homebrew/opt/openexr/lib/* ]] && [[ ! -f "$dep" ]]; then
      needs_reinstall=true
      break
    fi
  done < <(otool -L /opt/homebrew/opt/opencv/lib/libopencv_imgcodecs.dylib 2>/dev/null | awk 'NR>1 {print $1}')
fi
if $needs_reinstall; then
  echo "OpenCV/OpenEXR mismatch detected; rebuilding local OpenCV 4.13..."
  ./scripts/install-opencv-4.13-macos.sh
fi

if [ -d "${HOME}/.local/opencv-4.13.0/lib/cmake/opencv4" ]; then
  export OPENCV_DIR="${HOME}/.local/opencv-4.13.0/lib/cmake/opencv4"
elif [ -d "/usr/local/lib/cmake/opencv4" ]; then
  export OPENCV_DIR="/usr/local/lib/cmake/opencv4"
elif [ -d "/opt/homebrew/opt/opencv/lib/cmake/opencv4" ]; then
  export OPENCV_DIR="/opt/homebrew/opt/opencv/lib/cmake/opencv4"
elif [ -d "/usr/local/opt/opencv/lib/cmake/opencv4" ]; then
  export OPENCV_DIR="/usr/local/opt/opencv/lib/cmake/opencv4"
fi

echo "Done."
echo "  OPENCV_DIR=${OPENCV_DIR:-<set manually if needed>}"
echo "  cargo build --release"
echo "  make python"
