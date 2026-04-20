# rust:1.93-bookworm
FROM rust@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31 AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app/backend

FROM chef AS planner
COPY . /app/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev libpq-dev libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/backend/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . /app/
RUN cargo build --release --bin poziomki_backend-cli --bin worker

# debian:bookworm-slim
FROM debian@sha256:4724b8cc51e33e398f0e2e15e18d5ec2851ff0c2280647e1310bc1642182655d AS runtime-deps
SHELL ["/bin/sh", "-o", "pipefail", "-c"]
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq5 libwebp7 ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*

RUN set -eux; \
    mkdir -p /staging/usr/lib/x86_64-linux-gnu /staging/etc/ssl/certs; \
    cp -aL /etc/ssl/certs/ca-certificates.crt /staging/etc/ssl/certs/; \
    for soname in libpq.so.5 libwebp.so.7; do \
        cp -aL /usr/lib/x86_64-linux-gnu/${soname}* /staging/usr/lib/x86_64-linux-gnu/; \
    done; \
    for root in /staging/usr/lib/x86_64-linux-gnu/libpq.so.5 \
                /staging/usr/lib/x86_64-linux-gnu/libwebp.so.7; do \
        ldd "$root" | awk '/=>/ && $3 ~ /^\// { print $3 }'; \
    done | sort -u | while read -r dep; do \
        dest="/staging${dep}"; \
        [ -e "$dest" ] && continue; \
        mkdir -p "$(dirname "$dest")"; \
        cp -aL "$dep" "$dest"; \
    done

# gcr.io/distroless/cc-debian12:nonroot
FROM gcr.io/distroless/cc-debian12@sha256:e2d29aec8061843706b7e484c444f78fafb05bfe47745505252b1769a05d14f1 AS runtime
WORKDIR /app

COPY --from=runtime-deps /staging/ /
COPY --from=builder /app/backend/target/release/poziomki_backend-cli /usr/local/bin/poziomki_backend-cli
COPY --from=builder /app/backend/target/release/worker /usr/local/bin/worker

USER nonroot
EXPOSE 5150
ENTRYPOINT ["/usr/local/bin/poziomki_backend-cli"]
