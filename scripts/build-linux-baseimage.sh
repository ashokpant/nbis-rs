#!/usr/bin/env bash
# Build the Docker base image for Linux wheel builds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PLATFORM="${LINUX_PLATFORM:-linux/amd64}"
IMAGE="${NBIS_LINUX_BUILDER_IMAGE:-nbis-rs-linux-builder:24.04}"

echo "Building base image $IMAGE ($PLATFORM) ..."
docker build \
  --platform "$PLATFORM" \
  -f docker/nbis-rs-linux-builder.Dockerfile \
  -t "$IMAGE" \
  docker

echo "Done: $IMAGE"
echo "  make python-linux"
