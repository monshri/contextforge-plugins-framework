.PHONY: all build build-examples build-all-plugins build-test-plugins build-bench-plugins bench-all test \
        run-demos run-plugin-demo run-capabilities-demo run-sandbox-demo run-env-demo \
        run-token-attenuator-demo run-network-policy-demo setup-sandbox-demo-data clean clean-all help

PLUGIN_DIR    := ../cpex-wasm-plugin
WASM_DIR      := wasm
PLUGIN_TARGET := $(PLUGIN_DIR)/target/wasm32-wasip2/release/cpex_wasm_plugin.wasm

# Plugin lists — one binary is built per feature flag, then staged to wasm/.
# Keep these as the single source of truth; the plugin Makefile's PLUGINS list
# must stay in sync with DEMO_PLUGINS.
DEMO_PLUGINS  := identity-checker header-injector audit-logger token-attenuator \
                 tool-invoke-checker pii-guard audit-logger-custom remote-authz
TEST_PLUGINS  := fs-test net-test env-test noop fs-sandbox-demo env-sandbox-demo \
                 resource-test net-http-test
BENCH_PLUGINS := compute-bench noop

# ---------------------------------------------------------------------------
# Internal helper — build a list of plugins and stage each to wasm/.
# Usage: $(call build-plugins, var-name-holding-list)
# ---------------------------------------------------------------------------
define build-plugins
	@for plugin in $(1); do \
		echo "Building $$plugin..."; \
		cargo build --manifest-path $(PLUGIN_DIR)/Cargo.toml \
			--target wasm32-wasip2 --release \
			--features $$plugin --no-default-features; \
		cp $(PLUGIN_TARGET) $(WASM_DIR)/$$plugin.wasm; \
	done
	@echo "Done: $(2)"
endef

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------

all: build-all-plugins build-examples ## Build all plugins and host examples (default)

build-examples: ## Compile all host example binaries
	cargo build --examples

build-all-plugins: ## Build all demo WASM plugin binaries and stage to wasm/
	$(call build-plugins,$(DEMO_PLUGINS),All demo plugins built and staged.)

build-test-plugins: ## Build all test WASM plugins (fs-test, net-test, env-test, noop, ...)
	$(call build-plugins,$(TEST_PLUGINS),All test plugins built and staged.)

build-bench-plugins: ## Build benchmark WASM plugins (compute-bench, noop)
	$(call build-plugins,$(BENCH_PLUGINS),Benchmark plugins built and staged.)

# ---------------------------------------------------------------------------
# Run demos
# ---------------------------------------------------------------------------

run-demos: build-all-plugins build-examples ## Build all plugins and run all demos
	@echo "\n========== Plugin Demo (custom payload, pre/post invoke) ==========\n"
	cargo run --example wasm_plugin_demo
	@echo "\n========== Capabilities Demo (3 plugins, capability isolation) ==========\n"
	cargo run --example wasm_capabilities_demo
	@echo "\n========== All demos passed ==========\n"

run-plugin-demo: build-all-plugins build-examples ## Run the custom-payload plugin demo
	cargo run --example wasm_plugin_demo

run-capabilities-demo: build-all-plugins build-examples ## Run the capabilities isolation demo
	cargo run --example wasm_capabilities_demo

setup-sandbox-demo-data: ## Create examples/data subdirs and seed files for the fs sandbox demo
	@mkdir -p examples/data examples/data/rules examples/data/cache examples/data/audit \
		examples/data/counters examples/data/plugins examples/data/scratch
	@echo 'allow_tool: "*"' > examples/data/rules/policy.yaml
	@echo '0' > examples/data/counters/rate.txt
	@touch examples/data/plugins/fs-sandbox-demo.wasm
	@rm -rf examples/data/audit/events examples/data/scratch/work examples/data/counters/new
	@echo "Sandbox demo data ready under examples/data/"

run-sandbox-demo: build-test-plugins setup-sandbox-demo-data build-examples ## Run the filesystem sandbox permissions demo
	cargo run --example wasm_fs_sandbox_demo

run-env-demo: build-test-plugins build-examples ## Run the env variable sandbox permissions demo
	cargo run --example wasm_env_sandbox_demo

run-token-attenuator-demo: build-all-plugins build-examples ## Run the token delegation (token-attenuator) demo
	cargo run --example wasm_token_attenuator_demo

run-network-policy-demo: build-test-plugins build-examples ## Run the network policy sandbox demo (host/port/scheme/method enforcement)
	cargo run --example wasm_network_policy_demo

# ---------------------------------------------------------------------------
# Benchmarks
# ---------------------------------------------------------------------------

bench-all: build-all-plugins build-bench-plugins build-test-plugins ## Build all plugins and run benchmarks + generate chart
	cargo bench
	@echo "\n========== Generating performance chart ==========\n"
	cd benchmarking && python3 plot_results.py
	@echo "\n========== Benchmarks complete ==========\n"

# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------

test: clean-all build-all-plugins build-test-plugins ## Clean, rebuild all plugins, run all tests
	@echo "\n========== Running host tests ==========\n"
	cargo test
	@echo "\n========== Running demo plugin unit tests ==========\n"
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features identity-checker --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features header-injector --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features audit-logger --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features token-attenuator --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features tool-invoke-checker --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features pii-guard --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features audit-logger-custom --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features remote-authz --no-default-features
	@echo "\n========== Running test plugin unit tests ==========\n"
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features fs-sandbox-demo --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features env-sandbox-demo --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features resource-test --no-default-features
	cargo test --manifest-path $(PLUGIN_DIR)/Cargo.toml --features net-http-test --no-default-features
	@echo "\n========== All tests passed ==========\n"

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------

clean: ## Remove host build artifacts and wasm/ binaries
	cargo clean
	rm -f $(WASM_DIR)/*.wasm

clean-all: ## Remove host + plugin build artifacts and ALL .wasm files
	cargo clean
	cargo clean --manifest-path $(PLUGIN_DIR)/Cargo.toml
	cargo clean --manifest-path $(PLUGIN_DIR)/Cargo.toml --target wasm32-wasip2
	rm -f $(WASM_DIR)/*.wasm

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-24s\033[0m %s\n", $$1, $$2}'
