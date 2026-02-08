FROM rust:1.85-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --bin poziomki_backend-cli

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/poziomki_backend-cli /usr/local/bin/poziomki_backend-cli
COPY --from=builder /app/config ./config

EXPOSE 5150
CMD ["poziomki_backend-cli", "start", "-e", "production"]
