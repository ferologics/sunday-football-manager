alias b := build
alias c := check
alias r := run
alias w := watch

build:
  cargo build

clean:
  cargo clean

check:
  cargo check
  cargo clippy

run:
  cargo run

# Run with auto-reload on file changes (requires: cargo install cargo-watch)
watch:
  cargo watch -x run

# Build release binary
release:
  cargo build --release

# Build Docker image locally
docker-build:
  docker build -t sunday-manager .

# Run Docker container locally
docker-run:
  docker run -p 8000:8000 --env-file .env sunday-manager

setup:
  cargo install cargo-watch

test:
  cargo test
