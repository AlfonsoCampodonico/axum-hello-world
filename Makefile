IMAGE ?= axum-hello-world:dev
PORT  ?= 9115

.PHONY: run test fmt lint check docker docker-run

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

# Laravel Cloud runs amd64; pin the platform so a local arm64 build matches.
docker:
	docker build --platform linux/amd64 -t $(IMAGE) .

docker-run:
	docker run --rm -e PORT=$(PORT) -p $(PORT):$(PORT) $(IMAGE)
