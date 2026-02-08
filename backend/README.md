# Welcome to Loco :train:

[Loco](https://loco.rs) is a web and API framework running on Rust.

This is the **SaaS starter** which includes a `User` model and authentication based on JWT.
It also include configuration sections that help you pick either a frontend or a server-side template set up for your fullstack server.


## Quick Start

```sh
cargo loco start
```

```sh
$ cargo loco start
Finished dev [unoptimized + debuginfo] target(s) in 21.63s
    Running `target/debug/myapp start`

    :
    :
    :

controller/app_routes.rs:203: [Middleware] Adding log trace id

                      ▄     ▀
                                 ▀  ▄
                  ▄       ▀     ▄  ▄ ▄▀
                                    ▄ ▀▄▄
                        ▄     ▀    ▀  ▀▄▀█▄
                                          ▀█▄
▄▄▄▄▄▄▄  ▄▄▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄▄▄▄▄ ▄▄▄▄▄▄▄▄▄ ▀▀█
 ██████  █████   ███ █████   ███ █████   ███ ▀█
 ██████  █████   ███ █████   ▀▀▀ █████   ███ ▄█▄
 ██████  █████   ███ █████       █████   ███ ████▄
 ██████  █████   ███ █████   ▄▄▄ █████   ███ █████
 ██████  █████   ███  ████   ███ █████   ███ ████▀
   ▀▀▀██▄ ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀ ██▀
       ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
                https://loco.rs

environment: development
   database: automigrate
     logger: debug
compilation: debug
      modes: server

listening on http://localhost:5150
```

## Full Stack Serving

You can check your [configuration](config/development.yaml) to pick either frontend setup or server-side rendered template, and activate the relevant configuration sections.

## Local Infra (Postgres 18 + Caddy Upload Auth)

Start full local stack from repository root `docker-compose.yml`:

```sh
cd ..
docker compose -f docker-compose.yml up -d --build
```

`Caddyfile` is configured for upload auth flow targeting Garage S3:
- `GET /uploads/{file}` goes through Caddy.
- Caddy calls API `GET /api/v1/uploads/auth-check` via `forward_auth`.
- On success, Caddy proxies to Garage S3 path-style object endpoint (`/<bucket>/<key>`).

Use a custom Garage endpoint if needed:

```sh
GARAGE_S3_ENDPOINT=http://host.docker.internal:3900 \
GARAGE_S3_ACCESS_KEY=... \
GARAGE_S3_SECRET_KEY=... \
CADDY_GARAGE_S3_UPSTREAM=host.docker.internal:3900 \
GARAGE_S3_BUCKET=poziomki-uploads \
cd ..
docker compose -f docker-compose.yml up -d --build
```

Backend Garage env vars (production mode):
- `GARAGE_S3_ENDPOINT`
- `GARAGE_S3_BUCKET`
- `GARAGE_S3_ACCESS_KEY`
- `GARAGE_S3_SECRET_KEY`
- Optional: `GARAGE_S3_REGION`, `GARAGE_S3_URL_EXPIRY`, `GARAGE_S3_PUBLIC_URL`, `GARAGE_S3_VIRTUAL_HOST_STYLE`

## Quality Gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Additional static quality gate using [rust-code-analysis](https://github.com/mozilla/rust-code-analysis):

```sh
cargo install --locked rust-code-analysis-cli
./scripts/rust-code-analysis.sh
```

Thresholds can be tuned with env vars:

```sh
RCA_MIN_MI_VISUAL_STUDIO=12 RCA_MAX_CYCLOMATIC=6 ./scripts/rust-code-analysis.sh
```

## Getting help

Check out [a quick tour](https://loco.rs/docs/getting-started/tour/) or [the complete guide](https://loco.rs/docs/getting-started/guide/).
