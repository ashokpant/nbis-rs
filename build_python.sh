#!/usr/bin/env bash
set -euo pipefail

mkdir -p dist

if [ -f /etc/os-release ] && grep -qiE 'ubuntu|debian' /etc/os-release; then
  echo "Installing system dependencies..."
  sudo apt-get update
  sudo apt-get install -y python3.12-dev python3.12-venv build-essential
fi

VENV_DIR=".venv"
if [ ! -d "$VENV_DIR" ]; then
  echo "Creating Python 3.12 virtual environment..."
  python3.12 -m venv "$VENV_DIR"
fi

source "$VENV_DIR/bin/activate"

echo "Upgrading pip..."
pip install --upgrade pip setuptools wheel

echo "Installing maturin..."
pip install "maturin>=1.5,<2.0"

echo "Building Python wheel..."
maturin build --release --manifest-path Cargo.toml --out dist --strip

if [ -f "./patch_maturin_wheel.sh" ]; then
  echo "Patching wheel..."
  bash ./patch_maturin_wheel.sh
fi

echo "Validating wheel..."
pip install twine
twine check dist/nbis_py*.whl

echo "Build completed successfully"
ls -lh dist/*.whl