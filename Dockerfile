# ── Stage 1: Build Rust backend ────────────────────────────────────────────────
FROM rust:1.82-bookworm AS backend-builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY core/ core/
COPY nx_api/ nx_api/
COPY nx_cli/ nx_cli/
COPY nx_memory/ nx_memory/
COPY nx_session/ nx_session/
COPY nx_search/ nx_search/
COPY nx_mcp/ nx_mcp/
COPY nx_a2ui/ nx_a2ui/

RUN cargo build --release --bin nx_api

# ── Stage 2: Build Node frontend ──────────────────────────────────────────────
FROM node:20-bookworm-slim AS frontend-builder

WORKDIR /app
COPY nx_dashboard/package.json nx_dashboard/package-lock.json* ./nx_dashboard/
RUN cd nx_dashboard && npm ci --ignore-scripts

COPY nx_dashboard/ nx_dashboard/
RUN cd nx_dashboard && npm run build

# ── Stage 3: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy backend binary
COPY --from=backend-builder /app/target/release/nx_api /usr/local/bin/nx_api

# Copy frontend static files
COPY --from=frontend-builder /app/nx_dashboard/dist /app/static

# Copy seed workflow configs (if any)
COPY config/ /app/config/

# Default environment
ENV NEXUS_API_HOST=0.0.0.0
ENV NEXUS_API_PORT=8080
ENV NEXUS_DB_PATH=/data/nexus.db

EXPOSE 8080

VOLUME ["/data"]

CMD ["nx_api"]
