# Top-level task runner. Install `just` from https://github.com/casey/just.
# `just --list` to discover recipes.

set shell := ["bash", "-cu"]

# Run backend and frontend dev servers in parallel.
# Ctrl-C kills both via the recipe's process group.
run:
    #!/usr/bin/env bash
    set -u
    # Refuse to start if something is already on :8000 — usually a leaked
    # backend from a prior run that wasn't cleaned up. Better to fail loud
    # than silently talk to a stale binary.
    if ss -ltn 'sport = :8000' | grep -q LISTEN; then
        echo "Port 8000 is already in use. Run 'just kill' to clear stale dev servers." >&2
        exit 1
    fi
    # SIGHUP catches terminal close (window/SSH); SIGINT catches Ctrl-C;
    # SIGTERM catches `kill <pid>`. `kill 0` signals the whole process group.
    trap 'kill 0' SIGINT SIGTERM SIGHUP
    (cd backend && source .env; cargo run 2>&1 | sed -u 's/^/\x1b[31m[BACKEND]\x1b[0m  /') &
    (cd frontend && npm run dev 2>&1 | sed -u 's/^/\x1b[32m[FRNTEND]\x1b[0m /') &
    wait

# Kill any leaked dev servers from a previous `just run`.
kill:
    #!/usr/bin/env bash
    pkill -f 'target/(debug|release)/backend' || true
    pkill -f 'vite'                           || true
    echo "cleaned up"

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
