.PHONY: build release test run install install-debug clean lint fmt check

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
