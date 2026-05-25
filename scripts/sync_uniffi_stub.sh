#!/usr/bin/env bash
# Copy maturin-generated nbis.py into _uniffi_stubs/ for Linux wheel builds.
# Called automatically by make python (macOS) and make build [--if-present].
# Usage: ./scripts/sync_uniffi_stub.sh [--if-present]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STUB="$ROOT/bindings/python/nbis-python/_uniffi_stubs/nbis.py"
IF_PRESENT=false
if [ "${1:-}" = "--if-present" ]; then
  IF_PRESENT=true
fi

gen=$(find "$ROOT/target" -path '*/maturin/uniffi/nbis/nbis.py' 2>/dev/null | xargs ls -t 2>/dev/null | head -n 1 || true)

if [ -z "$gen" ] || [ ! -f "$gen" ]; then
  if $IF_PRESENT; then
    exit 0
  fi
  echo "No generated nbis.py under target/maturin/. Run 'make python' on macOS first." >&2
  exit 1
fi

mkdir -p "$(dirname "$STUB")"
cp "$gen" "$STUB"
echo "Synced UniFFI stub for Linux wheels: $STUB"
