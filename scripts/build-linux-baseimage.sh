#!/usr/bin/env bash
# Build the Docker base image for Linux wheel builds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PLATFORM="${LINUX_PLATFORM:-linux/amd64}"
ARCH="${PLATFORM#linux/}"
IMAGE="${NBIS_LINUX_BUILDER_IMAGE:-nbis-rs-linux-builder:24.04-${ARCH}}"

echo "Building base image $IMAGE ($PLATFORM) ..."
docker build \
  --platform "$PLATFORM" \
  -f docker/nbis-rs-linux-builder.Dockerfile \
  -t "$IMAGE" \
  .

echo "Done: $IMAGE"
echo "  LINUX_PLATFORM=$PLATFORM make python-linux"
