#!/usr/bin/env bash
# Verify a Linux wheel loads nbis without LD_LIBRARY_PATH / system OpenCV.
set -euo pipefail

WHEEL="${1:?wheel path}"
OPENCV_LIB_DIR="${OPENCV_LIB_DIR:-/usr/local/lib}"

if [ "$(uname -s)" != "Linux" ]; then
  echo "verify_bundled_linux_wheel.sh: Linux only"
  exit 1
fi

VENV="$(mktemp -d)"
trap 'rm -rf "$VENV"' EXIT

python3 -m venv "$VENV"
# shellcheck disable=SC1091
source "$VENV/bin/activate"
pip install -q "$WHEEL"

# Intentionally unset host OpenCV from the loader search path.
unset LD_LIBRARY_PATH

python - <<'PY'
import pathlib
import nbis.nbis.nbis as m

lib = pathlib.Path(m.__file__).with_name("libnbis.so")
print("libnbis:", lib)
assert lib.exists(), "libnbis.so missing"

from nbis import NbisExtractorSettings, new_nbis_extractor

settings = NbisExtractorSettings(
    min_quality=0.0,
    get_center=False,
    check_fingerprint=False,
    compute_nfiq2=True,
    ppi=500.0,
)
ext = new_nbis_extractor(settings)
data = pathlib.Path("test_data/p1/p1_1.png").read_bytes()
tpl = ext.extract_minutiae(data)
assert len(tpl.get()) > 0, "expected minutiae"
print("extract_minutiae OK:", len(tpl.get()), "minutiae")
PY

libnbis="$(find "$VENV" -name 'libnbis.so' | head -n 1)"
if ldd "$libnbis" 2>/dev/null | grep -q "${OPENCV_LIB_DIR}/libopencv"; then
  echo "FAIL: still linked to system OpenCV at ${OPENCV_LIB_DIR}"
  ldd "$libnbis" | grep opencv || true
  exit 1
fi

echo "PASS: bundled wheel works without LD_LIBRARY_PATH"
