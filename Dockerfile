FROM rust:latest as builder

WORKDIR /app

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY index.html ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Create app directory and data directory
RUN mkdir -p /app/data && chmod 755 /app/data

# Copy the binary
COPY --from=builder /app/target/release/fuel_cost_server /usr/local/bin/fuel_cost_server
RUN chmod +x /usr/local/bin/fuel_cost_server

# Copy static files if any
COPY --from=builder /app/index.html /app/

# Create a user and give ownership of /app/data
RUN useradd -r -s /bin/false appuser && \
    chown -R appuser:appuser /app/data

USER appuser
WORKDIR /app

EXPOSE 8880

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8880/ || exit 1

CMD ["fuel_cost_server"]