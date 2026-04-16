FROM rust:1.93-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app/backend

FROM chef AS planner
COPY . /app/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev libpq-dev libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/backend/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . /app/
RUN cargo build --release --bin poziomki_backend-cli --bin worker \
    && strip /app/backend/target/release/poziomki_backend-cli /app/backend/target/release/worker

FROM debian:bookworm-slim AS runtime-deps
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install -y --no-install-recommends libpq5 libwebp7 ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*

RUN set -eux; \
    mkdir -p /staging/usr/lib/x86_64-linux-gnu /staging/etc/ssl/certs; \
    cp -aL /etc/ssl/certs/ca-certificates.crt /staging/etc/ssl/certs/; \
    for lib in /usr/lib/x86_64-linux-gnu/libpq.so* \
               /usr/lib/x86_64-linux-gnu/libwebp.so* \
               /usr/lib/x86_64-linux-gnu/libsharpyuv.so*; do \
        [ -e "$lib" ] && cp -aL "$lib" /staging/usr/lib/x86_64-linux-gnu/ || true; \
    done; \
    for root in /staging/usr/lib/x86_64-linux-gnu/libpq.so* \
                /staging/usr/lib/x86_64-linux-gnu/libwebp.so*; do \
        [ -e "$root" ] || continue; \
        ldd "$root" | awk '/=>/ && $3 ~ /^\// { print $3 }'; \
    done | sort -u | while read -r dep; do \
        [ -f "$dep" ] && cp -aL "$dep" /staging/usr/lib/x86_64-linux-gnu/ || true; \
    done

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app

COPY --from=runtime-deps /staging/ /
COPY --from=ghcr.io/tarampampam/curl:8.6.0 /bin/curl /usr/local/bin/curl
COPY --from=builder /app/backend/target/release/poziomki_backend-cli /usr/local/bin/poziomki_backend-cli
COPY --from=builder /app/backend/target/release/worker /usr/local/bin/worker

USER nonroot
EXPOSE 5150
ENTRYPOINT ["/usr/local/bin/poziomki_backend-cli"]
