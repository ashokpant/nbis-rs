.PHONY: build build-release test python python-linux python-linux-aarch64 linux-baseimage wheels-all publish-check publish clean help install-linux install-macos

help:
	@echo "NBIS-rs"
	@echo "  make install-linux   Install system deps (Debian/Ubuntu)"
	@echo "  make install-macos   Install system deps (Homebrew)"
	@echo "  make build           Debug build"
	@echo "  make build-release   Release build"
	@echo "  make test            Run tests"
	@echo "  make python          Build host wheel"
	@echo "  make linux-baseimage Build/rebuild Docker image for python-linux"
	@echo "  make python-linux           Build Linux x86_64 wheel (Docker; OpenCV bundled)"
	@echo "  make python-linux-aarch64   Build Linux aarch64 wheel (Docker; OpenCV bundled)"
	@echo "  make wheels-all             Build macOS + Linux x86_64 + Linux aarch64 wheels"
	@echo "  make publish-check   Verify dist/ wheels (twine check)"
	@echo "  make publish         Upload dist/*.whl to PyPI (see docs/PUBLISHING.md)"
	@echo "  make clean           Remove build artifacts"

install-linux:
	./scripts/install-deps-linux.sh

install-macos:
	./scripts/install-deps-macos.sh

build:
	cargo build
	./scripts/sync_uniffi_stub.sh --if-present

build-release:
	cargo build --release --locked
	./scripts/sync_uniffi_stub.sh --if-present

test:
	cargo test --verbose

python:
	@test -d .venv || python3 -m venv .venv
	. .venv/bin/activate && pip install -q --upgrade pip "maturin>=1.5,<2.0" twine
	NBIS_PATH="$(PWD)/.venv/bin:$$PATH" ./scripts/build-python-wheel.sh

linux-baseimage:
	./scripts/build-linux-baseimage.sh

python-linux:
	LINUX_PLATFORM=linux/amd64 ./build_python_linux.sh

python-linux-aarch64:
	LINUX_PLATFORM=linux/arm64 ./build_python_linux.sh

wheels-all: python python-linux python-linux-aarch64
	./scripts/verify-dist-for-pypi.sh

publish-check:
	./scripts/verify-dist-for-pypi.sh
	@if [ -x .venv/bin/twine ]; then \
		.venv/bin/twine check dist/nbis_python*.whl; \
	else \
		twine check dist/nbis_python*.whl; \
	fi

publish: publish-check
	@if [ -x .venv/bin/twine ]; then \
		.venv/bin/twine upload --non-interactive dist/nbis_python*.whl; \
	else \
		twine upload --non-interactive dist/nbis_python*.whl; \
	fi

clean:
	cargo clean
	rm -rf dist/ dist/linux-amd64 dist/linux-arm64 .venv/ target/linux-docker-amd64 target/linux-docker-arm64
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

.SILENT: help
