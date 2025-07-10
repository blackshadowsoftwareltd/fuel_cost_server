FROM rust:latest as builder

WORKDIR /app

COPY Cargo.toml ./
COPY index.html ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/fuel_cost_server /usr/local/bin/fuel_cost_server

EXPOSE 8080

CMD ["fuel_cost_server"]

