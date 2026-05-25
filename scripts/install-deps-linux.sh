#!/usr/bin/env bash
# System dependencies for building nbis-rs on Debian/Ubuntu.
set -euo pipefail

sudo apt-get update
sudo apt-get install -y \
  build-essential \
  cmake \
  pkg-config \
  libopencv-dev \
  python3-dev \
  python3-venv

echo "Done."
echo "  cargo build --release"
echo "  make python   # maturin build --auditwheel=skip"
