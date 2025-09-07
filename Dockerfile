# ------- Build stage -------
FROM rust:1.83-slim AS builder
WORKDIR /app

# System deps required to compile (openssl, ring, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates build-essential && \
    rm -rf /var/lib/apt/lists/*

# Copy ALL sources and build directly to avoid stale cached dummy binaries
COPY . .
RUN cargo build --release --bin bookreview --bin seeder

# ------- Runtime stage -------
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates tzdata && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -ms /bin/sh appuser

# Binaries
COPY --from=builder /app/target/release/bookreview /app/bookreview
COPY --from=builder /app/target/release/seeder /app/seeder

# Runtime assets
COPY Rocket.toml .
COPY templates ./templates

# Create uploads directory
RUN mkdir -p /app/uploads && \
    chown -R appuser:appuser /app/uploads

ENV ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8000 \
    RUST_BACKTRACE=1 \
    UPLOADS_DIR=/app/uploads
EXPOSE 8000

USER appuser
CMD ["/app/bookreview"]