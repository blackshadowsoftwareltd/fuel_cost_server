FROM rust:latest as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY index.html ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install ALL possible runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    libssl-dev \
    pkg-config \
    libpq-dev \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/fuel_cost_server /usr/local/bin/fuel_cost_server

# Make sure it's executable
RUN chmod +x /usr/local/bin/fuel_cost_server

# TEST: Check if binary works
RUN ldd /usr/local/bin/fuel_cost_server || echo "Static binary"

# DON'T use non-root user for debugging
# RUN useradd -r -s /bin/false appuser
# USER appuser

EXPOSE 8880

# Remove health check for now
# HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
#   CMD curl -f http://localhost:8880/health || exit 1

# Add debug output
CMD ["sh", "-c", "echo 'Starting fuel_cost_server...' && /usr/local/bin/fuel_cost_server"]