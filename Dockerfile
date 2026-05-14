# syntax=docker/dockerfile:1.7

# ---------- Stage 1: build the frontend bundle ----------
FROM node:20-alpine AS frontend
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend ./
RUN npm run build
# /app/dist now contains index.html + assets/

# ---------- Stage 2: build the Rust binary in release mode ----------
FROM rust:1-bookworm AS backend
WORKDIR /src
# Cache deps separately from source for faster rebuilds.
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs \
    && cd backend && cargo build --release --quiet \
    && rm -rf src target/release/deps/backend* target/release/backend*
COPY backend ./backend
RUN cd backend && cargo build --release --quiet

# ---------- Stage 3: minimal runtime image ----------
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=backend  /src/backend/target/release/backend  /app/backend
COPY --from=backend  /src/backend/data                    /app/data
COPY --from=backend  /src/backend/templates               /app/templates
COPY --from=frontend /app/dist                            /app/public
# Rocket reads address/port from these env vars; ACA defaults to 8080.
ENV ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8080 \
    PUBLIC_DIR=/app/public
EXPOSE 8080
CMD ["/app/backend"]
