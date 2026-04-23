# rust:1.93-trixie
#
# NB: bumped from bookworm → trixie to pick up glibc ≥ 2.38. The `ort` crate
# (ONNX Runtime, used by the moderation engine) ships a pre-built
# `libonnxruntime.a` that references `__isoc23_strtol` / `__isoc23_strtoull`,
# which are only present in glibc 2.38+. bookworm ships glibc 2.36 and fails
# to link with `undefined symbol`.
FROM rust@sha256:f7c649bf7cc388826273a4977e8304778b7f8ad2e20838c1d52283cf23e49f7a AS chef
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

# debian:trixie-slim
FROM debian@sha256:e18da95f66066b7c5fa31491b524e83121271eca59a3d140f4906c8d0a090367 AS runtime-deps
# /bin/sh on bookworm-slim is dash, which doesn't support `-o pipefail`.
# Install bash first, then switch SHELL so the ldd pipeline below gets pipefail.
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq5 libwebp7 ca-certificates bash \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# hadolint ignore=DL4006
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

# gcr.io/distroless/cc-debian13:nonroot  (trixie-based — matches glibc 2.41)
FROM gcr.io/distroless/cc-debian13@sha256:951d10263d9a00fafaadb57615d3299811dde143fab6e64663685ee48c48137e AS runtime
WORKDIR /app

COPY --from=runtime-deps /staging/ /
COPY --from=builder /app/backend/target/release/poziomki_backend-cli /usr/local/bin/poziomki_backend-cli
COPY --from=builder /app/backend/target/release/worker /usr/local/bin/worker

# The int8 Bielik-Guard model (~125 MB) is NOT baked into the image — it's
# mounted read-only from the host at /app/moderation via docker-compose,
# mirroring the DKIM key pattern. See docs/moderation-deploy.md for the
# ops step, and scripts/guard-bench/export_onnx.py to regenerate.

USER nonroot
EXPOSE 5150
ENTRYPOINT ["/usr/local/bin/poziomki_backend-cli"]
