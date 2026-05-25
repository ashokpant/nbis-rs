.PHONY: build build-release test python clean help install-linux install-macos

help:
	@echo "NBIS-rs"
	@echo "  make install-linux   Install system deps (Debian/Ubuntu)"
	@echo "  make install-macos   Install system deps (Homebrew)"
	@echo "  make build           Debug build"
	@echo "  make build-release   Release build"
	@echo "  make test            Run tests"
	@echo "  make python          Install deps + build Python wheel"
	@echo "  make clean           Remove build artifacts"

install-linux:
	@chmod +x scripts/install-deps-linux.sh 2>/dev/null || true
	./scripts/install-deps-linux.sh

install-macos:
	@chmod +x scripts/install-deps-macos.sh 2>/dev/null || true
	./scripts/install-deps-macos.sh

build:
	cargo build

build-release:
	cargo build --release --locked

test:
	cargo test --verbose

python:
	@chmod +x build_python.sh scripts/*.sh patch_maturin_wheel.sh 2>/dev/null || true
	./build_python.sh

clean:
	cargo clean
	rm -rf dist/ .venv/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

.SILENT: help
