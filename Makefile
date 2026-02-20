APP=solana_infinity_engine

.PHONY: fmt clippy test build run release

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

build:
	cargo build

run:
	cargo run

release:
	cargo build --release
