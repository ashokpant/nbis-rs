#!/usr/bin/env bash
# Runs inside Ubuntu Docker; builds nbis-python wheel into dist/linux/.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  cmake \
  curl \
  libopencv-dev \
  pkg-config \
  python3 \
  python3-pip \
  python3-venv \
  unzip \
  zip

if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
fi
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

if [ ! -d /io ]; then
  echo "Error: /io not mounted."
  exit 1
fi

cd /io
export CARGO_TARGET_DIR=/io/target/linux-docker
mkdir -p /io/dist/linux "$CARGO_TARGET_DIR"

python3 -m venv /tmp/build-venv
# shellcheck disable=SC1091
source /tmp/build-venv/bin/activate
pip install --upgrade pip
pip install "maturin>=1.5,<2.0" twine

maturin build \
  --release \
  --manifest-path Cargo.toml \
  --out dist/linux \
  --strip \
  --locked \
  --auditwheel=skip

MATURIN_DIST_DIR=dist/linux WHEEL_GLOB='nbis_python*.whl' bash ./patch_maturin_wheel.sh
twine check dist/linux/nbis_python*.whl

ls -1 dist/linux/nbis_python*.whl
