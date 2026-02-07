# Poziomki RS

Monorepo for the Poziomki platform: Loco (Rust) backend + KMP Compose Multiplatform mobile app.

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

### Commands

```bash
cd mobile
./gradlew :composeApp:assembleDebug   # build Android debug APK
```

## Conventions

- Atomic conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, etc.)
- Use git worktrees for parallel work
