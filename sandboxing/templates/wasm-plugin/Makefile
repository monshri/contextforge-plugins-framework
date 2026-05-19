PLUGIN_NAME := "plugin"
TARGET := wasm32-wasip2

.PHONY: all build component verify clean help run-host

all: component

help:
	@echo "Targets:"
	@echo "  build     - Compile the plugin to WASM"
	@echo "  component - Build and copy the final .wasm component"
	@echo "  verify    - Show WIT exports of the built component"
	@echo "  run-host  - Build plugin and run the host example"
	@echo "  clean     - Remove build artifacts"

build:
	cargo build --target $(TARGET) --release

component: build
	cp target/$(TARGET)/release/$(subst -,_,$(PLUGIN_NAME)).wasm $(PLUGIN_NAME).wasm
	@echo "Built: $(PLUGIN_NAME).wasm"
	@ls -lh $(PLUGIN_NAME).wasm

verify: component
	wasm-tools component wit $(PLUGIN_NAME).wasm

run-host: component
	@echo "Running host example..."
	cargo run --example host --release

clean:
	cargo clean
	rm -f $(PLUGIN_NAME).wasm
