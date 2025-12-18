alias b := build
alias c := check
alias r := run
alias d := deploy

build:
  cargo build

check:
  cargo check
  cargo clippy

deploy:
  cargo shuttle deploy

init:
  cargo shuttle login

run:
  cargo shuttle run

setup:
  cargo install cargo-shuttle

test:
  cargo test
