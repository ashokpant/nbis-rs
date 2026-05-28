#!/usr/bin/env bash
# Ensure dist/ contains macOS + Linux wheels for the version in pyproject.toml.
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
has_linux=false
for w in "${wheels[@]}"; do
  case "$(basename "$w")" in
    *macosx*|*darwin*) has_mac=true ;;
    *manylinux*) has_linux=true ;;
  esac
  echo "  $(basename "$w") ($(du -h "$w" | cut -f1))"
done

$has_mac || { echo "Missing macOS wheel" >&2; exit 1; }
$has_linux || { echo "Missing Linux manylinux wheel (run: make python-linux)" >&2; exit 1; }

echo "OK: nbis-python $VERSION ready for PyPI ($((${#wheels[@]})) wheels)"
