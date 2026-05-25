.PHONY: build build-release test python android clean help

help:
	@echo "NBIS-rs Build System"
	@echo "make build              Build in debug mode"
	@echo "make build-release      Build in release mode"
	@echo "make test               Run tests"
	@echo "make python             Build Python bindings"
	@echo "make android            Build Android bindings"
	@echo "make clean              Clean build artifacts"

build:
	cargo build

build-release:
	cargo build --release --locked

test:
	cargo test --verbose

python:
	./build_python.sh

android:
	./build_android.sh

clean:
	cargo clean
	rm -rf dist/
	rm -rf .venv/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true

.SILENT: help
