.PHONY: build release test run install install-debug clean lint fmt check setup dev

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

run:
	cargo run --release -- $(ARGS)

install:
	-ez daemon stop
	cargo install --locked --path .
	ez daemon start

install-debug:
	-ez daemon stop
	cargo install --locked --debug --path .
	ez daemon start

clean:
	cargo clean

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check:
	cargo fmt -- --check
	cargo clippy --locked -- -D warnings
	cargo test --locked

# Run a debug build against an isolated dev config/data dir (.dev/ by default).
# Usage: make dev [ARGS="config get selector.backend"] [DEV_DIR=/path]
# Seeds <DEV_DIR>/config/config.toml once from your real config (repos and
# sessions are NOT copied) and selects the native browser backend.
DEV_DIR ?= $(CURDIR)/.dev
REAL_CONFIG := $(if $(XDG_CONFIG_HOME),$(XDG_CONFIG_HOME),$(HOME)/.config)/ez/config.toml
ifeq ($(shell uname -s),Darwin)
REAL_CONFIG := $(HOME)/Library/Application Support/ez/config.toml
endif

dev:
	cargo build
	@mkdir -p "$(DEV_DIR)/config" "$(DEV_DIR)/data"
	@if [ ! -f "$(DEV_DIR)/config/config.toml" ]; then \
		if [ -f "$(REAL_CONFIG)" ]; then cp "$(REAL_CONFIG)" "$(DEV_DIR)/config/config.toml"; echo "dev: seeded config from $(REAL_CONFIG)"; fi; \
		EZ_NO_DAEMON=1 EZ_CONFIG_DIR="$(DEV_DIR)/config" EZ_DATA_DIR="$(DEV_DIR)/data" ./target/debug/ez config set selector.backend native; \
	fi
	EZ_CONFIG_DIR="$(DEV_DIR)/config" EZ_DATA_DIR="$(DEV_DIR)/data" ./target/debug/ez $(ARGS)

setup:
ifeq ($(OS),Windows_NT)
	winget install --id Git.Git -e --accept-source-agreements --accept-package-agreements
	winget install --id stedolan.jq -e --accept-source-agreements --accept-package-agreements
else
	UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
	brew install jq
else
	sudo apt-get install -y jq
endif
endif
