Move fast towards MVP, competition is growing.

## Quality Gates

- Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Rust metrics: run `backend/scripts/rust-code-analysis.sh` and keep it passing.
- Kotlin: `./gradlew ktlintCheck detekt` in `mobile/`.
- Treat warnings as errors; do not merge if any quality gate fails.
- Never bypass checks in CI; fix code or adjust thresholds in PR with justification.

## Deploy

- **APK deploy:** `/deploy-apk` — builds debug APK, uploads via `scp poziomki:/var/www/download/poziomki-rs-debug.apk`, install link: `https://mobile.poziomki.app/download/poziomki-rs-debug.apk`
- **Backend deploy:** NixOS via Colmena from `infra/` — `colmena apply --on poziomki-prod`
- **Server:** `ssh poziomki` (ubuntu user, key auth). Caddy reverse-proxies `rs.poziomki.app` (API:5150), `cdn-rs.poziomki.app` (Garage:3900), `chat.poziomki.app` (Tuwunel:6167), `mobile.poziomki.app` (static + API:3000).
