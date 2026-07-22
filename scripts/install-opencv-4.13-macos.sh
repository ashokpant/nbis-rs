#!/usr/bin/env bash
# Build OpenCV 4.13.0 into a user prefix for macOS (Homebrew is OpenCV 5-only now).
# NFIQ2 requires find_package(OpenCV 4), so brew opencv 5.x cannot be used.
set -euo pipefail

OPENCV_VERSION="${OPENCV_VERSION:-4.13.0}"
OPENCV_PREFIX="${OPENCV_PREFIX:-${HOME}/.local/opencv-${OPENCV_VERSION}}"
OPENCV_BUILD_DIR="${OPENCV_BUILD_DIR:-/tmp/opencv-${OPENCV_VERSION}-macos-build}"
OPENCV_SOURCE_DIR="${OPENCV_SOURCE_DIR:-/tmp/opencv-${OPENCV_VERSION}-macos-src}"
JOBS="${JOBS:-$(sysctl -n hw.ncpu 2>/dev/null || echo 4)}"

if [ -f "${OPENCV_PREFIX}/lib/cmake/opencv4/OpenCVConfig.cmake" ]; then
  ver="$(PKG_CONFIG_PATH="${OPENCV_PREFIX}/lib/pkgconfig:${PKG_CONFIG_PATH:-}" \
    pkg-config --modversion opencv4 2>/dev/null || true)"
  if [[ "${ver}" == 4.13.* ]] || [ -z "${ver}" ]; then
    echo "OpenCV ${ver:-4.13} already installed under ${OPENCV_PREFIX}"
    echo "  export OPENCV_DIR=${OPENCV_PREFIX}/lib/cmake/opencv4"
    exit 0
  fi
fi

command -v cmake >/dev/null || { echo "cmake required" >&2; exit 1; }
command -v curl >/dev/null || { echo "curl required" >&2; exit 1; }

mkdir -p "${OPENCV_BUILD_DIR}" "${OPENCV_SOURCE_DIR}" "${OPENCV_PREFIX}"
if [[ ! -f "${OPENCV_SOURCE_DIR}/CMakeLists.txt" ]]; then
  curl -fsSL "https://github.com/opencv/opencv/archive/refs/tags/${OPENCV_VERSION}.tar.gz" \
    | tar -xz -C /tmp
  rm -rf "${OPENCV_SOURCE_DIR}"
  mv "/tmp/opencv-${OPENCV_VERSION}" "${OPENCV_SOURCE_DIR}"
fi

cmake -S "${OPENCV_SOURCE_DIR}" -B "${OPENCV_BUILD_DIR}" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX="${OPENCV_PREFIX}" \
  -DOPENCV_GENERATE_PKGCONFIG=ON \
  -DBUILD_SHARED_LIBS=ON \
  -DBUILD_LIST=core,imgproc,ml,imgcodecs \
  -DBUILD_TESTS=OFF \
  -DBUILD_PERF_TESTS=OFF \
  -DBUILD_EXAMPLES=OFF \
  -DBUILD_opencv_apps=OFF \
  -DWITH_GTK=OFF \
  -DWITH_QT=OFF \
  -DWITH_OPENGL=OFF \
  -DWITH_FFMPEG=OFF \
  -DWITH_OPENCL=OFF

cmake --build "${OPENCV_BUILD_DIR}" -j"${JOBS}"
cmake --install "${OPENCV_BUILD_DIR}"

echo "OpenCV ${OPENCV_VERSION} installed to ${OPENCV_PREFIX}"
echo "  export OPENCV_DIR=${OPENCV_PREFIX}/lib/cmake/opencv4"
echo "  export PKG_CONFIG_PATH=${OPENCV_PREFIX}/lib/pkgconfig:\${PKG_CONFIG_PATH:-}"
echo "  export DYLD_LIBRARY_PATH=${OPENCV_PREFIX}/lib:\${DYLD_LIBRARY_PATH:-}"
