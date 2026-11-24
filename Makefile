.PHONY: build release test run install install-debug clean lint fmt check setup dev dev-sync

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
# sessions are NOT copied; use `make dev-sync`) and selects the native backend.
DEV_DIR ?= $(CURDIR)/.dev
REAL_CONFIG_DIR := $(if $(XDG_CONFIG_HOME),$(XDG_CONFIG_HOME),$(HOME)/.config)/ez
REAL_DATA_DIR := $(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share)/ez
ifeq ($(shell uname -s),Darwin)
REAL_CONFIG_DIR := $(HOME)/Library/Application Support/ez
REAL_DATA_DIR := $(REAL_CONFIG_DIR)
endif
REAL_CONFIG := $(REAL_CONFIG_DIR)/config.toml

dev:
	cargo build
	@mkdir -p "$(DEV_DIR)/config" "$(DEV_DIR)/data"
	@if [ ! -f "$(DEV_DIR)/config/config.toml" ]; then \
		if [ -f "$(REAL_CONFIG)" ]; then cp "$(REAL_CONFIG)" "$(DEV_DIR)/config/config.toml"; echo "dev: seeded config from $(REAL_CONFIG)"; fi; \
		EZ_NO_DAEMON=1 EZ_CONFIG_DIR="$(DEV_DIR)/config" EZ_DATA_DIR="$(DEV_DIR)/data" ./target/debug/ez config set selector.backend native; \
	fi
	EZ_CONFIG_DIR="$(DEV_DIR)/config" EZ_DATA_DIR="$(DEV_DIR)/data" ./target/debug/ez $(ARGS)

# Mirror your real registered repos, sessions and notes into the dev dirs,
# replacing what's there. Re-run to pick up repos/sessions added since.
# Sessions still point at your REAL worktrees: deleting/renaming them in dev
# changes the real worktree/branch but only the dev index is updated.
dev-sync:
	@mkdir -p "$(DEV_DIR)/config" "$(DEV_DIR)/data"
	@if [ ! -d "$(REAL_CONFIG_DIR)/repos" ]; then echo "dev-sync: no repos at $(REAL_CONFIG_DIR)/repos"; exit 1; fi
	@rm -rf "$(DEV_DIR)/config/repos" && cp -R "$(REAL_CONFIG_DIR)/repos" "$(DEV_DIR)/config/repos"
	@if [ -d "$(REAL_DATA_DIR)/repos" ]; then rm -rf "$(DEV_DIR)/data/repos" && cp -R "$(REAL_DATA_DIR)/repos" "$(DEV_DIR)/data/repos"; fi
	@echo "dev-sync: copied repos/sessions/notes into $(DEV_DIR) (sessions point at your real worktrees)"

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
