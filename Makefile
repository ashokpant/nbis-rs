.PHONY: build build-release test python python-linux linux-baseimage publish clean help install-linux install-macos

help:
	@echo "NBIS-rs"
	@echo "  make install-linux   Install system deps (Debian/Ubuntu)"
	@echo "  make install-macos   Install system deps (Homebrew)"
	@echo "  make build           Debug build"
	@echo "  make build-release   Release build"
	@echo "  make test            Run tests"
	@echo "  make python          Build wheel + sync _uniffi_stubs (macOS → Linux)"
	@echo "  make linux-baseimage Build/rebuild Docker image for python-linux"
	@echo "  make python-linux    Build nbis-python wheel (Docker base image)"
	@echo "  make publish         Upload dist/nbis_python-*.whl to PyPI"
	@echo "  make clean           Remove build artifacts"

install-linux:
	@chmod +x scripts/install-deps-linux.sh 2>/dev/null || true
	./scripts/install-deps-linux.sh

install-macos:
	@chmod +x scripts/install-deps-macos.sh 2>/dev/null || true
	./scripts/install-deps-macos.sh

build:
	cargo build
	@chmod +x scripts/sync_uniffi_stub.sh 2>/dev/null || true
	@./scripts/sync_uniffi_stub.sh --if-present

build-release:
	cargo build --release --locked
	@chmod +x scripts/sync_uniffi_stub.sh 2>/dev/null || true
	@./scripts/sync_uniffi_stub.sh --if-present

test:
	cargo test --verbose

python:
	@chmod +x build_python.sh scripts/*.sh patch_maturin_wheel.sh 2>/dev/null || true
	./build_python.sh

linux-baseimage:
	@chmod +x scripts/build-linux-baseimage.sh 2>/dev/null || true
	./scripts/build-linux-baseimage.sh

python-linux:
	@chmod +x build_python_linux.sh scripts/*.sh patch_maturin_wheel.sh 2>/dev/null || true
	./build_python_linux.sh

publish:
	@if [ ! -d .venv ]; then echo "Run make python first"; exit 1; fi
	.venv/bin/twine upload --non-interactive --skip-existing dist/nbis_python*.whl

clean:
	cargo clean
	rm -rf dist/ dist/linux .venv/ target/linux-docker
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

.SILENT: help
