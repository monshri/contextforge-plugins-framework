.PHONY: all build-plugin build-host build clean install-deps run

# Default target
all: build

# Install all dependencies
install:
	@echo "Installing wasm32-wasip2 target..."
	rustup target add wasm32-wasip2
	@echo "Installing cargo-component..."
	cargo install cargo-component

# Build plugin only
build:
	@echo "Building plugin..."
	cargo component build --target wasm32-wasip2 --release

# Clean all build artifacts
clean:
	cargo clean
	cd plugin && cargo clean

# Check both projects
check:
	cargo check
	cd plugin && cargo check --target wasm32-wasip2

# Run tests
test:
	cargo test
	cd plugin && cargo test

# Show build info
info:
	@echo "=== Host ==="
	@echo "Name: sandboxing"
	@echo "Output: target/release/sandboxing"
	@echo ""
	@echo "=== Plugin ==="
	@echo "Name: plugin"
	@echo "Target: wasm32-wasip2"
	@echo "Output: plugin/target/wasm32-wasip2/release/plugin.wasm"
