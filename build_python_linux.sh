#!/usr/bin/env bash
# Build nbis-python wheel for Linux (Ubuntu 24.04) via Docker.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required. On Linux use: make python"
  exit 1
fi

PLATFORM="${LINUX_PLATFORM:-linux/amd64}"
IMAGE="${LINUX_IMAGE:-ubuntu:24.04}"

mkdir -p "$ROOT/dist/linux"

echo "Building Linux wheel in $IMAGE ($PLATFORM) ..."

docker run --rm \
  --platform "$PLATFORM" \
  -v "$ROOT:/io" \
  -v nbis-rs-cargo-registry:/root/.cargo/registry \
  -v nbis-rs-cargo-git:/root/.cargo/git \
  -w /io \
  "$IMAGE" \
  bash /io/scripts/docker-build-python-inner.sh

shopt -s nullglob
linux_wheels=(dist/linux/nbis_python*.whl)
if [ ${#linux_wheels[@]} -eq 0 ]; then
  echo "No wheel found in dist/linux/"
  exit 1
fi
for wheel in "${linux_wheels[@]}"; do
  cp -f "$wheel" dist/
  echo "Copied to dist/$(basename "$wheel")"
done
rm -rf dist/linux

echo "Built: dist/nbis_python-*linux*.whl"
