check:
	cargo check --workspace --all-targets
	cargo check --workspace --all-features --lib --target wasm32-unknown-unknown

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all

clippy-fix:
	cargo clippy --fix --allow-dirty --workspace --all-targets --all-features -- -D warnings -W clippy::all

test:
	cargo test --workspace --all-targets --all-features
	cargo test --workspace --doc

docs:
	cargo doc --no-deps

build:
	trunk build

build-release:
	trunk build --release

serve:
	trunk serve

clean:
	cargo clean
	rm -rf dist

# Print help information
help:
	@echo "Available commands:"
	@echo "  make              - Run check, test, and build"
	@echo "  make check        - Check code compilation"
	@echo "  make format       - Format code with rustfmt"
	@echo "  make format-check - Verify code formatting"
	@echo "  make clippy       - Run clippy for linting"
	@echo "  make test         - Run all tests"
	@echo "  make docs         - Generate documentation"
	@echo "  make build        - Build project with trunk"
	@echo "  make build-release - Build project with trunk in release mode"
	@echo "  make serve        - Serve project with trunk"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make help         - Show this help message"
