FROM rust:1.88-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release --bin poziomki_backend-cli --bin worker

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libwebp7 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -r appuser && useradd -r -g appuser -d /app -s /sbin/nologin appuser \
    && chown appuser:appuser /app

COPY --from=builder /app/target/release/poziomki_backend-cli /usr/local/bin/poziomki_backend-cli
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
COPY --from=builder --chown=appuser:appuser /app/config ./config

USER appuser
EXPOSE 5150
CMD ["poziomki_backend-cli"]
