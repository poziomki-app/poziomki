# Poziomki RS

Move fast towards MVP, competition is growing. Monorepo for the Poziomki platform: Loco (Rust) backend + KMP Compose Multiplatform mobile app.

## Structure

- `backend/` — Loco REST API (Rust, SeaORM, PostgreSQL)
- `mobile/` — KMP Compose Multiplatform (Android + iOS)

## Backend

- Framework: [Loco](https://loco.rs/) with SeaORM
- Database: PostgreSQL
- Auth: JWT (built-in Loco auth scaffold)
- Dependencies: matrix-sdk, ruma, opendal

### Commands

```bash
cd backend
cargo loco start          # run dev server
cargo loco db migrate     # run migrations
cargo loco generate ...   # scaffold models/controllers/etc.
cargo test                # run tests
```

## Mobile

- Framework: KMP Compose Multiplatform
- Package: `com.poziomki.app`
- **API URL:** `apiBaseUrl` in `mobile/gradle.properties` (production: `https://rs.poziomki.app`). Override per-build with `-PapiBaseUrl=...`. Default fallback is `http://localhost:5150` — **never ship localhost to devices**.

### Commands

```bash
cd mobile
./gradlew :composeApp:assembleDebug   # build Android debug APK
```

### Local testing (Waydroid)

Build, install, and restart in one shot (requires `ANDROID_HOME` set and Waydroid session running):

```bash
cd mobile && ANDROID_HOME=~/Android/Sdk ./gradlew :composeApp:assembleDebug && waydroid app install composeApp/build/outputs/apk/debug/composeApp-x86_64-debug.apk
```

Waydroid must be restarted after install: `waydroid session stop && waydroid session start`.

## Deploy

### APK (Android)

Use `/deploy-apk` skill or manually:

```bash
cd mobile && ./gradlew :composeApp:assembleDebug
scp mobile/composeApp/build/outputs/apk/debug/composeApp-arm64-v8a-debug.apk \
    poziomki:/var/www/download/poziomki-rs-debug.apk
```

Install link: `https://mobile.poziomki.app/download/poziomki-rs-debug.apk`

Server: `ssh poziomki` — Caddy serves `/download/*` from `/var/www/download/`.

## Conventions

- Atomic conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, etc.)
- Use git worktrees for parallel work

## Quality Rules
- Rust: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Rust metrics: run `backend/scripts/rust-code-analysis.sh` and keep it passing.
- Kotlin: `./gradlew ktlintCheck detekt` in `mobile/`.
- Treat warnings as errors; do not merge if any quality gate fails.
- Never bypass checks in CI; fix code or adjust thresholds in PR with justification.
