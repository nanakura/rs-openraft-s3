# https://github.com/casey/just

set dotenv-load := true

fmt:
    cargo fmt --all
    cargo sort

check:
    cargo check
    cargo clippy -- -D warnings

test:
    cargo test --all-features -- --test-threads=1

dev: fmt check test

install:
    cargo install --features binary --path .