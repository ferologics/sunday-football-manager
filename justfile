alias b := build
alias c := check
alias r := run
alias w := watch

build:
  cargo build

check:
  cargo check
  cargo clippy

clean:
  cargo clean

# Build Docker image locally
docker-build:
  docker build -t football-manager .

# Run Docker container locally
docker-run:
  docker run -p 8000:8000 --env-file .env football-manager

# Build release binary
release:
  cargo build --release

run:
  cargo run

setup:
  cargo install cargo-watch

test:
  cargo test

# Run with auto-reload on file changes (requires: cargo install cargo-watch)
watch:
  cargo watch -x run
