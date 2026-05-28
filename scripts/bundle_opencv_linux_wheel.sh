#!/usr/bin/env bash
# Vendor OpenCV 4.13 shared libs into a Linux wheel; set $ORIGIN RPATH on libnbis.so.
# Usage: bundle_opencv_linux_wheel.sh WHEEL_PATH [OUTPUT_DIR]
set -euo pipefail

[ "$(uname -s)" = "Linux" ] || { echo "Linux only" >&2; exit 1; }
command -v patchelf >/dev/null || { echo "patchelf required" >&2; exit 1; }

WHEEL_IN="$(realpath "${1:?wheel}")"
OUT_DIR="$(realpath -m "${2:-$(dirname "$WHEEL_IN")}")"
OPENCV_LIB_DIR="${OPENCV_LIB_DIR:-/usr/local/lib}"
mkdir -p "$OUT_DIR"

is_system_lib() {
  case "$1" in
    /lib/*|/lib64/*|/usr/lib/*|/usr/lib64/*) return 0 ;;
  esac
  case "$(basename "$1")" in
    ld-linux*|linux-vdso*|libc.so*|libm.so*|libdl.so*|libpthread.so*|librt.so*|libresolv.so*|libnss_*|libutil.so*|libgcc_s.so*|libstdc++.so*|libatomic.so*)
      return 0 ;;
  esac
  return 1
}

should_vendor() {
  is_system_lib "$1" && return 1
  case "$1" in *libopencv*|"$OPENCV_LIB_DIR"/*) return 0 ;; esac
  return 1
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
unzip -q "$WHEEL_IN" -d "$work"

libnbis="$(find "$work" -name libnbis.so -print -quit)"
[ -n "$libnbis" ] || { echo "libnbis.so not in wheel" >&2; exit 1; }
pkg_dir="$(dirname "$libnbis")"

declare -A seen=()
queue=("$libnbis")
while [ "${#queue[@]}" -gt 0 ]; do
  bin="${queue[0]}"
  queue=("${queue[@]:1}")
  [ -f "$bin" ] || continue
  key="$(realpath "$bin")"
  [ -n "${seen[$key]:-}" ] && continue
  seen[$key]=1
  while IFS= read -r dep; do
    [ -f "$dep" ] || continue
    should_vendor "$dep" || continue
    dest="$pkg_dir/$(basename "$dep")"
    if [ ! -f "$dest" ]; then
      cp -L "$dep" "$dest"
      queue+=("$dest")
    fi
  done < <(ldd "$bin" 2>/dev/null | awk '/=>/ {print $3}')
done

patchelf --set-rpath '$ORIGIN' "$libnbis"
for so in "$pkg_dir"/*.so*; do
  [ -f "$so" ] && patchelf --set-rpath '$ORIGIN' "$so" 2>/dev/null || true
done

out="$OUT_DIR/$(basename "$WHEEL_IN")"
rm -f "$out"
( cd "$work" && zip -qr "$out" . )

if ! compgen -G "${pkg_dir}/libopencv_"*.so* >/dev/null; then
  echo "ERROR: no libopencv_*.so in wheel" >&2
  exit 1
fi

if [ "$(realpath "$out")" != "$WHEEL_IN" ]; then
  cp -f "$out" "$WHEEL_IN"
fi
echo "Bundled OpenCV into $(basename "$WHEEL_IN") ($(du -h "$WHEEL_IN" | cut -f1))"
