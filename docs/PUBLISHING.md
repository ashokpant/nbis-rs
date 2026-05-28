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
#    e.g. 0.1.14 → git tag v0.1.14 && git push origin v0.1.14

# 2. Build both platform wheels
make wheels-all          # macOS wheel + Linux wheel (Docker, OpenCV bundled on Linux)

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
pip install "nbis-python>=0.1.14"
```

**pyproject.toml**

```toml
dependencies = [
    "nbis-python>=0.1.14",
]
```

**uv**

```toml
dependencies = ["nbis-python>=0.1.14"]
```

```bash
uv add "nbis-python>=0.1.14"
uv sync
```

### Platform notes

| Platform | Wheel on PyPI | Runtime |
|----------|----------------|---------|
| Linux x86_64 (`manylinux_2_28`) | Yes | OpenCV **bundled** in wheel |
| macOS arm64 / x86_64 | Yes | Needs **Homebrew OpenCV** ≥ 4.13 (`brew install opencv`) |

Linux Docker apps do **not** need `install-opencv-4.13-linux.sh` when using ≥ 0.1.14.

### Example: `fingerprintlib` / services

```toml
# fingerprintlib/pyproject.toml
dependencies = [
    "nbis-python>=0.1.14",
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

In **fingerprintlib** (until PyPI has 0.1.14):

```bash
cd ../nbis-rs && make wheels-all
cd ../fingerprintlib && make sync-dev
```

Or install a built wheel directly:

```bash
pip install /path/to/nbis-rs/dist/nbis_python-0.1.14-....whl
```
