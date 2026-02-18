# Simplification Plan (MVP-Focused, Feature-Parity)

## Goal
Remove **5k-10k+ lines** while keeping behavior, performance, and security.

## Baseline (measured)
- Backend Rust source (`backend/src`): `~8,439` lines.
- Backend migrations: `~1,476` lines.
- Backend tests: `~783` lines.
- Mobile Kotlin source (`mobile/composeApp/src` + `mobile/shared/src`): `~17,469` lines.
- Tracked vendored Rust crates (`backend/vendor`): `~11,151` lines.

## Priority 0: Validate biggest cut first

### 1) Remove vendored patched crates (if patches are no longer required)
- Files:
  - `backend/vendor/selectors-patched/**`
  - `backend/vendor/sea-orm-migration-patched/**`
  - `backend/Cargo.toml` (`[patch.crates-io]` block)
- Estimated reduction: **~11,000 lines**.
- Risk:
  - Medium: depends on whether patches are still needed.
- Guardrails:
  - Confirm patch rationale and reproduce without patches.
  - Run all quality gates before merge.

## Backend simplification

### 2) Remove legacy auth compatibility paths
- Files:
  - `backend/src/controllers/migration_api/mod.rs` (legacy OTP aliases)
  - `backend/src/controllers/migration_api/state_auth.rs` (legacy token fallback/migration path)
  - `backend/src/app.rs` (`migrate_legacy_session_tokens` boot hook)
- Estimated reduction: **~150 lines**.
- Note:
  - Do this only after explicit one-time data migration for legacy session tokens.

### 3) Remove dead Loco scaffold in users model + dead tests
- Files:
  - `backend/src/models/users.rs`
  - `backend/tests/models/users.rs`
  - `backend/tests/workers/mod.rs`
  - `backend/tests/tasks/mod.rs`
- Estimated reduction: **~480 lines**.
- Scope:
  - Remove unused magic-link/reset-password/verification-token paths and tests covering only removed code.

### 4) Consolidate duplicated backend DTOs (no codegen project)
- Files:
  - `backend/src/controllers/migration_api/state_types.rs`
- Estimated reduction: **~200 lines**.
- Scope:
  - Merge near-duplicate response types (`ProfileResponse`/`FullProfileResponse`/recommendation variants, session/tag variants) where wire-compatibility allows.
- Note:
  - Avoid introducing OpenAPI/codegen infrastructure in this pass.

### 5) Reduce backend module indirection with targeted merges
- Files:
  - `backend/src/controllers/migration_api/events*.rs`
  - `backend/src/controllers/migration_api/uploads*.rs`
  - `backend/src/controllers/migration_api/auth*.rs`
- Estimated reduction: **~350 lines**.
- Scope:
  - Merge only thin split files where boundaries are artificial.
  - Preserve public routes and behavior.

### 6) Simplify visibility noise (`pub(in crate::controllers::migration_api)`)
- Files:
  - `backend/src/controllers/migration_api/state_types.rs`
  - optionally related state files where safe.
- Estimated reduction: **~200 lines** (mostly wrapping/noise).
- Scope:
  - Prefer `pub(super)` item visibility plus normal field visibility where module boundaries already enforce encapsulation.

### 7) Rename `migration_api` to `api` (readability)
- Files:
  - `backend/src/controllers/migration_api/**` → `backend/src/controllers/api/**`
  - imports/references across backend.
- Estimated reduction: **0 lines**.
- Benefit:
  - Removes stale migration naming and improves readability.

## Mobile simplification

### 8) Extract shared profile form primitives
- Files:
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/profile/ProfileEditScreen.kt`
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/onboarding/ProfileSetupScreen.kt`
- Estimated reduction: **~500 lines**.
- Scope:
  - Create shared form/content components and keep screen-specific orchestration thin.

### 9) Add shared form/VM simplifications
- Files:
  - Event screens: `EventCreateScreen.kt`, `EventDetailScreen.kt`
  - Main list VMs: `EventsViewModel`, `ExploreViewModel`, `MessagesViewModel` (+ related)
- Estimated reduction: **~700 lines** (combined target for shared forms + list base patterns).
- Scope:
  - Extract shared event presentation/form sections.
  - Extract `BaseListViewModel` pattern for repeated loading/error/filter/list flow.

### 10) Deduplicate offline sync write paths (realistic scope)
- Files:
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRepository.kt`
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/ProfileRepository.kt`
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/sync/SyncEngine.kt`
- Estimated reduction: **~150 lines**.
- Scope:
  - Extract shared DB upsert/apply methods used by both optimistic updates and replay.

### 11) API model dedup (mobile, no codegen)
- Files:
  - `mobile/shared/src/commonMain/kotlin/com/poziomki/app/api/Models.kt`
- Estimated reduction: **~200 lines**.
- Scope:
  - Merge obvious duplicate model families where behavior and API parsing remain unchanged.

## Realistic totals
- Conservative without vendor removal: **~2,700 lines**.
- With vendor removal: **~13,700 lines**.

## Execution order
1. Verify and execute vendor removal.
2. Remove dead backend model/test code.
3. Legacy auth cleanup.
4. Backend DTO/visibility/indirection cleanup.
5. Mobile shared forms + BaseListViewModel.
6. Mobile sync/model dedup.
7. Optional readability rename `migration_api` → `api`.

## Quality-gate integration
- Every PR must pass:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `backend/scripts/rust-code-analysis.sh`
  - `cd mobile && ./gradlew ktlintCheck detekt`
- Treat current failing gates as part of refactor acceptance criteria, not deferred cleanup.

## PR slicing
- Keep each PR under ~500-900 changed lines.
- One simplification axis per PR.
- No feature removals hidden as simplification.
