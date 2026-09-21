PORT ?= 9115
BIN  ?= target/release/axum-hello-world

.PHONY: run test fmt lint check build serve

run:
	cargo run

test:
	cargo test

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

# Everything CI checks, in one command.
check:
	cargo fmt --all -- --check
	cargo clippy --all-targets -- -D warnings
	cargo test

# `build` and `serve` mirror the build and deploy commands configured on the
# Laravel Cloud environment, so the platform build can be reproduced locally.
build:
	cargo build --release --locked

serve: build
	PORT=$(PORT) $(BIN)
