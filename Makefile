.PHONY: build release test install clean lint fmt check

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

install:
	cargo install --locked --path .

clean:
	cargo clean

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

check:
	cargo fmt -- --check
	cargo clippy -- -D warnings
	cargo test
