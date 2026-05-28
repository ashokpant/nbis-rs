#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

command -v docker >/dev/null || {
  echo "Docker required (or run: make python on Linux)" >&2
  exit 1
}

IMAGE="${NBIS_LINUX_BUILDER_IMAGE:-nbis-rs-linux-builder:24.04}"
PLATFORM="${LINUX_PLATFORM:-linux/amd64}"

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  LINUX_PLATFORM="$PLATFORM" NBIS_LINUX_BUILDER_IMAGE="$IMAGE" \
    bash "$ROOT/scripts/build-linux-baseimage.sh"
fi

mkdir -p dist/linux

docker run --rm --platform "$PLATFORM" \
  -v "$ROOT:/io" \
  -v nbis-rs-cargo-registry:/root/.cargo/registry \
  -v nbis-rs-cargo-git:/root/.cargo/git \
  -w /io \
  "$IMAGE" \
  env NBIS_DIST_DIR=/io/dist/linux \
      NBIS_CARGO_TARGET_DIR=/io/target/linux-docker \
      NBIS_SKIP_HOST_DEPS=1 \
      NBIS_SKIP_STUB_SYNC=1 \
      NBIS_PATH=/opt/nbis-build-venv/bin:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
      OPENCV_LIB_DIR=/usr/local/lib \
      bash /io/scripts/build-python-wheel.sh

wheel="$(ls -t dist/linux/nbis_python*.whl | head -n 1)"
cp -f "$wheel" dist/
echo "Built: dist/$(basename "$wheel")"
rm -rf dist/linux
