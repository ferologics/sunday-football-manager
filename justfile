alias b := build
alias c := check
alias f := format
alias r := run
alias w := watch

build:
  cargo build

# Build release binary
build-release:
  cargo build --release

# Extract changelog section for a version
changelog VERSION:
  ./scripts/changelog.sh {{VERSION}}

check: format
  cargo check
  cargo clippy

format:
  cargo fmt

clean:
  cargo clean

# Build Docker image locally
docker-build:
  docker build -t football-manager .

# Run Docker container locally
docker-run:
  docker run -p 8000:8000 --env-file .env football-manager

# Create and push a git tag from Cargo.toml version, triggering GH release
release:
  #!/usr/bin/env bash
  VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
  echo "Releasing $VERSION..."
  echo "Changelog:"
  ./scripts/changelog.sh "$VERSION"
  echo ""
  read -p "Create tag and push? [y/N] " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    git tag "$VERSION"
    git push origin "$VERSION"
  fi

run:
  cargo run

setup:
  cargo install cargo-watch

test:
  cargo test

# Run with auto-reload on file changes (requires: cargo install cargo-watch)
watch:
  cargo watch -- cargo run
