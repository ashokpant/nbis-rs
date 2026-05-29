# Publishing `nbis-python` to PyPI

Package name on PyPI: **`nbis-python`** (import: `nbis`).

## One-time setup

1. [PyPI account](https://pypi.org/account/register/) and [API token](https://pypi.org/manage/account/token/) (scope: entire account or project `nbis-python`).
2. Configure credentials locally:

```bash
# ~/.pypirc (recommended)
[pypi]
username = __token__
password = pypi-AgEIcHlwaS5vcmcCJ...   # token value only

# Or env vars for CI / one-off uploads
export TWINE_USERNAME=__token__
export TWINE_PASSWORD=pypi-...
```

3. GitHub repo secret **`PYPI_API_TOKEN`** (for automated publish on tag).

## Release steps (maintainer)

```bash
# 1. Bump version in pyproject.toml + Cargo.toml (keep in sync), commit, tag
#    e.g. 0.1.15 → git tag v0.1.15 && git push origin v0.1.15

# 2. Build platform wheels (macOS + Linux x86_64 + Linux aarch64)
make wheels-all

# 3. Verify artifacts
make publish-check

# 4. Upload to PyPI
make publish
```

Or trigger **GitHub Actions → Release** workflow on tag `v*.*.*` (builds wheels + GitHub Release).  
For PyPI upload, use **Publish to PyPI** workflow (see `.github/workflows/pypi-publish.yml`).

## Installing in another application

After publish, depend on PyPI only (no path to `../nbis-rs`):

**pip**

```bash
pip install "nbis-python>=0.1.15"
```

**pyproject.toml**

```toml
dependencies = [
    "nbis-python>=0.1.15",
]
```

**uv**

```toml
dependencies = ["nbis-python>=0.1.15"]
```

```bash
uv add "nbis-python>=0.1.15"
uv sync
```

### Platform notes

| Platform | Wheel on PyPI | Runtime |
|----------|----------------|---------|
| Linux x86_64 (`manylinux_2_28`) | Yes | OpenCV **bundled** in wheel |
| Linux aarch64 (`manylinux_2_28`) | Yes (≥ 0.1.15) | OpenCV **bundled** in wheel |
| macOS arm64 / x86_64 | Yes | Needs **Homebrew OpenCV** ≥ 4.13 (`brew install opencv`) |

Linux Docker apps do **not** need `install-opencv-4.13-linux.sh` when using ≥ 0.1.14.

### Linux aarch64 (Graviton, Raspberry Pi 64-bit, etc.)

PyPI wheels for **0.1.14 and earlier** do not include Linux aarch64. Use **≥ 0.1.15**, or build locally on the machine:

```bash
cd nbis-rs
make install-linux   # native aarch64 only; installs OpenCV 4.13 to /usr/local
make python          # produces dist/nbis_python-*-manylinux_*_aarch64.whl
pip install dist/nbis_python-*-manylinux_*_aarch64.whl
```

From macOS with Docker (cross-build):

```bash
make python-linux-aarch64
```

### Example: `fingerprintlib` / services

```toml
# fingerprintlib/pyproject.toml
dependencies = [
    "nbis-python>=0.1.15",
    # ...
]
```

Remove any `[tool.uv.sources]` override for `nbis-python` after the version is on PyPI, then:

```bash
cd fingerprintlib
uv lock --upgrade-package nbis-python
uv sync
```

## Local development before publish

Build wheels and install from `dist/`:

```bash
cd nbis-rs && make wheels-all
pip install dist/nbis_python-*-$(python -c "import sysconfig; print(sysconfig.get_platform())").whl
```

Or install a built wheel directly:

```bash
pip install /path/to/nbis-rs/dist/nbis_python-0.1.15-....whl
```
