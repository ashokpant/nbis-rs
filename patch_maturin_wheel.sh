#!/bin/bash
# Fix wheel METADATA for twine; ensure nbis/nbis/nbis.py is present (Linux UniFFI quirk).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
NBIS_PY_SRC="$ROOT/bindings/python/nbis-python/_uniffi_stubs/nbis.py"

DIST_DIR=${MATURIN_DIST_DIR:-dist}
WHEEL_GLOB=${WHEEL_GLOB:-nbis_python*.whl}

pushd "$DIST_DIR" > /dev/null

wheel_file=$(ls -t $WHEEL_GLOB 2>/dev/null | head -n 1)
wheel_file=${wheel_file:-$(ls -t *.whl | head -n 1)}
wheel_path=$(realpath "$wheel_file")
wheel_unzip_dir=$(mktemp -d)

unzip -q "$wheel_file" -d "$wheel_unzip_dir"

metadata_file=$(find "$wheel_unzip_dir" -name METADATA)
tmpfile=$(mktemp)
grep -v '^License-File:' "$metadata_file" > "$tmpfile"
mv "$tmpfile" "$metadata_file"

# Linux builds may omit UniFFI-generated Python; ensure package stub and bindings exist.
nbis_pkg=$(find "$wheel_unzip_dir" -type d -path '*/nbis/nbis' | head -n 1)
if [ -n "$nbis_pkg" ]; then
  if [ ! -f "$nbis_pkg/nbis.py" ] && [ -f "$NBIS_PY_SRC" ]; then
    cp "$NBIS_PY_SRC" "$nbis_pkg/nbis.py"
  fi
  if [ ! -s "$nbis_pkg/__init__.py" ] 2>/dev/null; then
    printf '%s\n' 'from .nbis import *  # noqa: F403' > "$nbis_pkg/__init__.py"
  fi
fi

rm "$wheel_file"
cd "$wheel_unzip_dir"
shopt -s dotglob
zip -qr "$wheel_path" *

popd > /dev/null
rm -rf "$wheel_unzip_dir"

echo "Patched wheel: $DIST_DIR/$(basename "$wheel_path")"
