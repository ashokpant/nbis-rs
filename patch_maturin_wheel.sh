#!/bin/bash
# Fix wheel METADATA for twine (remove duplicate License-File entries).
set -euo pipefail

pushd dist > /dev/null

wheel_file=$(ls *.whl | head -n 1)
wheel_path=$(realpath "$wheel_file")
wheel_unzip_dir=$(mktemp -d)

unzip -q "$wheel_file" -d "$wheel_unzip_dir"

metadata_file=$(find "$wheel_unzip_dir" -name METADATA)
tmpfile=$(mktemp)
grep -v '^License-File:' "$metadata_file" > "$tmpfile"
mv "$tmpfile" "$metadata_file"

rm "$wheel_file"
cd "$wheel_unzip_dir"
shopt -s dotglob
zip -qr "$wheel_path" *

popd > /dev/null
rm -rf "$wheel_unzip_dir"

echo "Patched wheel: dist/$(basename "$wheel_path")"
