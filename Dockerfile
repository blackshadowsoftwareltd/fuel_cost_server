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
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/fuel_cost_server /usr/local/bin/fuel_cost_server

# Create a non-root user for security
RUN useradd -r -s /bin/false appuser
USER appuser

# FIX: Use port 8880 to match your Rust server
EXPOSE 8880

# FIX: Health check on port 8880
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8880/health || exit 1

CMD ["fuel_cost_server"]