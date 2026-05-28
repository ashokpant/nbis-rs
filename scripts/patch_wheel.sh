#!/usr/bin/env bash
# Fix wheel METADATA for twine; ensure nbis/nbis/nbis.py exists (Linux UniFFI quirk).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STUB="$ROOT/bindings/python/nbis-python/_uniffi_stubs/nbis.py"
DIST_DIR="${MATURIN_DIST_DIR:-dist}"
WHEEL_GLOB="${WHEEL_GLOB:-nbis_python*.whl}"

wheel_file="$(ls -t "$DIST_DIR"/$WHEEL_GLOB 2>/dev/null | head -n 1)"
[ -n "$wheel_file" ] || { echo "No wheel in $DIST_DIR" >&2; exit 1; }
wheel_file="$(realpath "$wheel_file")"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

unzip -q "$wheel_file" -d "$work"

metadata_file="$(find "$work" -name METADATA -print -quit)"
grep -v '^License-File:' "$metadata_file" > "${metadata_file}.tmp"
mv "${metadata_file}.tmp" "$metadata_file"

nbis_pkg="$(find "$work" -type d -path '*/nbis/nbis' | head -n 1)"
if [ -n "$nbis_pkg" ]; then
  [ -f "$nbis_pkg/nbis.py" ] || [ ! -f "$STUB" ] || cp "$STUB" "$nbis_pkg/nbis.py"
  [ -s "$nbis_pkg/__init__.py" ] 2>/dev/null || printf '%s\n' 'from .nbis import *  # noqa: F403' > "$nbis_pkg/__init__.py"
fi

rm -f "$wheel_file"
( cd "$work" && zip -qr "$wheel_file" . )

echo "Patched: $wheel_file"
