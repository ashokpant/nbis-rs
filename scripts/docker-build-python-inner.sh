#!/usr/bin/env bash
# Runs inside nbis-rs-linux-builder image; expects /io mounted from repo root.
set -euo pipefail

if [ ! -d /io ]; then
  echo "Error: /io not mounted."
  exit 1
fi

cd /io
export CARGO_TARGET_DIR=/io/target/linux-docker
mkdir -p /io/dist/linux "$CARGO_TARGET_DIR"

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
