#!/usr/bin/env bash
# Ensure dist/ contains macOS + Linux (x86_64 + aarch64) wheels for the version in pyproject.toml.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(grep -E '^version = ' pyproject.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')"
DIST="${DIST_DIR:-dist}"

shopt -s nullglob
wheels=("$DIST"/nbis_python-"$VERSION"-*.whl)
if [ ${#wheels[@]} -eq 0 ]; then
  echo "No wheels for nbis-python $VERSION in $DIST" >&2
  echo "Run: make wheels-all" >&2
  exit 1
fi

has_mac=false
has_linux_x86=false
has_linux_aarch64=false
for w in "${wheels[@]}"; do
  case "$(basename "$w")" in
    *macosx*|*darwin*) has_mac=true ;;
    *manylinux*x86_64*) has_linux_x86=true ;;
    *manylinux*aarch64*) has_linux_aarch64=true ;;
  esac
  echo "  $(basename "$w") ($(du -h "$w" | cut -f1))"
done

$has_mac || { echo "Missing macOS wheel (run: make python)" >&2; exit 1; }
$has_linux_x86 || { echo "Missing Linux x86_64 wheel (run: make python-linux)" >&2; exit 1; }
$has_linux_aarch64 || { echo "Missing Linux aarch64 wheel (run: make python-linux-aarch64)" >&2; exit 1; }

echo "OK: nbis-python $VERSION ready for PyPI ($((${#wheels[@]})) wheels)"
