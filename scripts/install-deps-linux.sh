#!/usr/bin/env bash
# System dependencies for building nbis-rs on Debian/Ubuntu.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

sudo apt-get update
sudo apt-get install -y \
  build-essential \
  cmake \
  curl \
  git \
  libjpeg-dev \
  libpng-dev \
  libtiff-dev \
  pkg-config \
  python3-dev \
  python3-venv

sudo bash "${ROOT}/scripts/install-opencv-4.13-linux.sh"

echo "Done."
echo "  export OPENCV_DIR=/usr/local/lib/cmake/opencv4"
echo "  export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:\$PKG_CONFIG_PATH"
echo "  cargo build --release"
echo "  make python   # maturin build --auditwheel=skip"
