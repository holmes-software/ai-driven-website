# Top-level task runner. Install `just` from https://github.com/casey/just.
# `just --list` to discover recipes.

set shell := ["bash", "-cu"]

# Run backend and frontend dev servers in parallel.
# Ctrl-C kills both via the recipe's process group.
run:
    #!/usr/bin/env bash
    set -u
    # `kill 0` signals the entire process group (this script + children),
    # so Ctrl-C reliably tears down `cargo run` and `vite` together.
    trap 'kill 0' SIGINT SIGTERM
    (cd backend && cargo run 2>&1 | sed -u 's/^/\x1b[31m[BACKEND]\x1b[0m  /') &
    (cd frontend && npm run dev 2>&1 | sed -u 's/^/\x1b[32m[FRNTEND]\x1b[0m /') &
    wait

# Run backend tests, then frontend tests.
test:
    cd backend && cargo test
    cd frontend && npm test

# Build both projects.
build:
    cd backend && cargo build --release
    cd frontend && npm run build

# Format both projects in place.
fmt:
    cd backend && cargo fmt
    cd frontend && npm run fmt

# Check that both projects are formatted (no changes written).
fmt-check:
    cd backend && cargo fmt -- --check
    cd frontend && npm run fmt:check
