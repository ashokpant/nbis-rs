#!/usr/bin/env bash
# Build and install OpenCV 4.13.0 to /usr/local (shared libs + pkg-config).
# Used by install-deps-linux.sh and the Linux Docker builder image.
set -euo pipefail

OPENCV_VERSION="${OPENCV_VERSION:-4.13.0}"
OPENCV_PREFIX="${OPENCV_PREFIX:-/usr/local}"
OPENCV_BUILD_DIR="${OPENCV_BUILD_DIR:-/tmp/opencv-${OPENCV_VERSION}-build}"
OPENCV_SOURCE_DIR="${OPENCV_SOURCE_DIR:-/tmp/opencv-${OPENCV_VERSION}-src}"

need_version() {
  if command -v pkg-config >/dev/null 2>&1; then
    local ver
    ver="$(PKG_CONFIG_PATH="${OPENCV_PREFIX}/lib/pkgconfig:${PKG_CONFIG_PATH:-}" \
      pkg-config --modversion opencv4 2>/dev/null || true)"
    if [[ "${ver}" == 4.13.* ]]; then
      echo "OpenCV ${ver} already installed under ${OPENCV_PREFIX}"
      return 0
    fi
  fi
  return 1
}

if need_version; then
  exit 0
fi

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Run as root or with sudo (installs to ${OPENCV_PREFIX})" >&2
  exit 1
fi

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  cmake \
  curl \
  git \
  libjpeg-dev \
  libpng-dev \
  libtiff-dev \
  pkg-config \
  unzip

mkdir -p "${OPENCV_BUILD_DIR}" "${OPENCV_SOURCE_DIR}"
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
  -DWITH_OPENGL=OFF

cmake --build "${OPENCV_BUILD_DIR}" -j"$(nproc)"
cmake --install "${OPENCV_BUILD_DIR}"

ldconfig

echo "OpenCV ${OPENCV_VERSION} installed to ${OPENCV_PREFIX}"
echo "  export OPENCV_DIR=${OPENCV_PREFIX}/lib/cmake/opencv4"
echo "  export PKG_CONFIG_PATH=${OPENCV_PREFIX}/lib/pkgconfig:\$PKG_CONFIG_PATH"
echo "  export LD_LIBRARY_PATH=${OPENCV_PREFIX}/lib:\$LD_LIBRARY_PATH"
