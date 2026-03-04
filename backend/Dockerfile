FROM rust:1.93-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev libpq-dev libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin poziomki_backend-cli --bin worker

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libpq5 libwebp7 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r appuser && useradd -r -g appuser -d /app -s /sbin/nologin appuser \
    && chown appuser:appuser /app

COPY --from=builder /app/target/release/poziomki_backend-cli /usr/local/bin/poziomki_backend-cli
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
USER appuser
EXPOSE 5150
CMD ["poziomki_backend-cli"]
