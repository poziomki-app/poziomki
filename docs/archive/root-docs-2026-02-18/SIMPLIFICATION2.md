# Codebase Simplification Plan

**Baseline:**

| Area | LOC | Notes |
|------|-----|-------|
| Backend src/ | 8,439 | Production code |
| Backend tests/ | 783 | |
| Backend migration/ | 1,476 | |
| Backend vendor/ | 11,151 | Patched crates (tracked) |
| Mobile src/ (excl. build/) | 17,469 | |
| **Total tracked** | **~39,300** | |

---

## Part A — Line deletion (remove code that adds no product value)

### A1. Remove vendored patched crates (~11,000 lines)

`backend/vendor/` contains two full crate forks: `selectors-patched` and `sea-orm-migration-patched`, referenced via `[patch.crates-io]` in `Cargo.toml`.

**Action:** Investigate whether the patches are still needed. If the upstream crates now cover the use case, delete both directories and the patch block.

**Verify:** `cargo build`, `cargo test`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `backend/scripts/rust-code-analysis.sh`.

**Savings:** ~11,000 lines if both removable, proportionally less if only one.

### A2. Remove dead Loco user model methods (~200 lines)

`models/users.rs` (377 lines) is 60% unused scaffold. None of these are called from any controller:

- `find_by_verification_token` — OTP flow, not email-link verification
- `find_by_magic_token` / `create_magic_link` / `clear_magic_link` — magic link auth not implemented
- `find_by_reset_token` / `set_forgot_password_sent` / `reset_password` — no password reset flow
- `set_email_verification_sent` — unused
- `find_by_api_key` on `impl Model` — duplicate of `impl Authenticable`
- `LoginParams` struct — unused (controllers use `SignInBody`)
- `Validator` + `Validatable` impl — not used at controller level

**Verify:** `grep -r` each function name in `backend/src/controllers/` returns no hits. Confirm with `cargo build`.

**Action:** Strip to ~120 lines: keep `find_by_email`, `find_by_pid`, `verify_password`, `create_with_password`, `generate_jwt`, `ActiveModelBehavior`.

### A3. Remove dead tests (~280 lines)

`tests/models/users.rs` (360 lines) — most tests exercise dead model methods from A2:
- `can_verification_token` — tests `set_email_verification_sent` (unused)
- `can_set_forgot_password_sent` — tests `set_forgot_password_sent` (unused)
- `can_reset_password` — tests `reset_password` (unused)
- `magic_link` — tests `create_magic_link` (unused)

Keep: `test_can_validate_model`, `can_create_with_password`, `can_find_by_email`, `can_find_by_pid`.

Also delete empty files: `tests/workers/mod.rs`, `tests/tasks/mod.rs`.

**Verify:** `cargo test` passes after deletion.

### A4. Merge duplicate mobile API model types (~100 lines)

`Models.kt` (381 lines) has near-identical pairs:
- `Profile` vs `ProfileWithTags` — identical except `tags` field

**Action:** Use `tags: List<Tag> = emptyList()` on a single `Profile` type. Update call sites. Same pattern for any event duplicates.

**Verify:** `./gradlew :composeApp:assembleDebug`, `./gradlew ktlintCheck detekt`.

### A5. Dead code cleanup (~30 lines)

- Unused imports across mobile screens (run `./gradlew ktlintCheck` to find them)
- Empty module files

**Verify:** Lint passes.

**Part A total: ~11,600 lines** (with vendor removal) or **~600 lines** (without).

---

## Part B — Complexity reduction (simplify logic and structure, verified savings)

### B1. Remove dead legacy auth paths (~150 lines)

- `migration_api/mod.rs` — duplicate OTP route aliases
- `state_auth.rs` — legacy token migration fallback at boot
- `app.rs` — `migrate_legacy_session_tokens` call

**Prerequisite:** Confirm mobile only uses `/api/v1/auth/verify-otp` and `/api/v1/auth/resend-otp`. Run one explicit DB migration for any remaining legacy session tokens.

**Verify:** `cargo test`, contract tests in `tests/requests/migration_contract.rs` pass.

### B2. Consolidate duplicate response types (~80 lines)

`state_types.rs` (429 lines) has near-duplicate struct families:
- `ProfileResponse` vs `FullProfileResponse` vs `ProfileRecommendation` — 3 structs with 90% field overlap
- `MatchingTagResponse` vs `EventTagResponse` vs `TagResponse` — 3 tag types
- `SessionView` vs `SessionListItem`

**Action:** Consolidate to: `ProfileResponse` (with optional `tags`, optional `score`), one `TagResponse`, one `SessionView`. Use `#[serde(skip_serializing_if = "Option::is_none")]`.

**Verify:** Contract tests pass, `cargo clippy`, manual API response shape check.

### B3. Extract shared profile form (mobile, ~400 lines)

`ProfileEditScreen.kt` (1,003 lines) and `ProfileSetupScreen.kt` (434 lines) share ~50% of their UI: image gallery, tag picker, bio editor, gradient picker, program/name/age fields.

**Action:** Extract `ProfileFormContent` composable (~400 lines). Both screens become thin wrappers (~150 and ~100 lines respectively).

**Verify:** `./gradlew :composeApp:assembleDebug`, `./gradlew ktlintCheck detekt`. Manual test both edit and onboarding flows.

### B4. Extract shared event form (mobile, ~150 lines)

`EventCreateScreen.kt` (731 lines) and `EventDetailScreen.kt` (267 lines) duplicate date formatting, location display, and tag display.

**Action:** Extract shared composables for date/location/tag display sections. EventDetail reuses them in read-only mode.

**Verify:** Same as B3. Manual test create + detail screens.

### B5. Extract BaseListViewModel (mobile, ~150 lines)

`EventsViewModel`, `ExploreViewModel`, `MessagesViewModel` all repeat: loading state, error state, filter application, list fetch → emit.

**Action:** Create `BaseListViewModel<T, Filter>` with shared state management. Each concrete VM shrinks by ~50 lines.

**Verify:** `./gradlew :composeApp:assembleDebug`. Manual test each list screen (events, explore, messages).

### B6. Merge over-split backend event files (~100 lines)

Events span 6 files (1,192 lines). Heavy cross-file imports.

**Action:** Merge `events_update.rs` (152) into `events_mutations.rs`. Merge `events_tags.rs` (120) into `events_support.rs`. Result: 4 files instead of 6.

**Verify:** `cargo build`, `cargo test`, `cargo clippy`.

**Note:** Do not force-merge all into fewer files if it creates >500-line hotspots that fail rust-code-analysis thresholds.

### B7. Merge small backend auth/upload files (~60 lines)

- Merge `auth_session.rs` (57 lines) into `auth.rs`
- Merge `uploads_support.rs` (93 lines) into `uploads.rs`

**Verify:** `cargo build`, `cargo test`, `cargo clippy`.

### B8. Simplify chat bubble rendering (mobile, ~80 lines)

`MessageEventRow.kt` (341 lines) duplicates `Surface` + `BubbleContent` for mine vs other messages.

**Action:** Unify into single code path with `isMine` controlling alignment, color, and avatar visibility.

**Verify:** Build + manual test chat screen.

### B9. Simplify AppNavigation onboarding boilerplate (mobile, ~50 lines)

`AppNavigation.kt` (491 lines) repeats `getBackStackEntry` + `koinViewModel(viewModelStoreOwner=...)` 3 times.

**Action:** Extract helper function.

**Verify:** Build + manual test onboarding flow.

**Part B total: ~1,200 lines.**

---

## Part C — Readability improvements (low line savings, high clarity)

### C1. Rename `migration_api` → `api`

The name is legacy from migrating off the old TS backend. It's now the only API.

**Action:** `mv controllers/migration_api controllers/api`, update all `use` paths, `mod.rs` declarations.

**Verify:** `cargo build`, `cargo test`, full `cargo clippy`.

### C2. Simplify visibility modifiers after C1

After rename, `pub(in crate::controllers::migration_api)` (204 occurrences in `state_types.rs` alone) becomes `pub(in crate::controllers::api)`. Consider further simplifying to `pub(super)` + `pub` fields where the module boundary already enforces encapsulation.

**Note:** This is noise reduction, not logic simplification. Don't count toward savings target. Only do if it doesn't fight tooling (e.g., if clippy recommends the longer form, keep it).

### C3. Extract dialogs from ProfileEditScreen

Move `BioEditorDialog` and `GradientPickerDialog` to separate files in `ui/component/`. No line savings, but the 1,003-line file becomes navigable.

### C4. Reusable OverlayTopBar composable

`EventChatScreen` and `ProfilePreview` both have overlay navigation bars on images. Extract shared composable.

---

## Summary

| # | Item | Lines Saved | Risk | Verification |
|---|------|------------|------|-------------|
| A1 | Vendor removal | ~11,000 | Low (if patches unneeded) | cargo build/test/clippy |
| A2 | Dead user model methods | ~200 | Low | grep + cargo build |
| A3 | Dead tests + empty files | ~280 | Low | cargo test |
| A4 | Duplicate mobile models | ~100 | Low | build + lint |
| A5 | Dead code cleanup | ~30 | Trivial | lint |
| B1 | Legacy auth paths | ~150 | Medium (needs DB migration) | contract tests |
| B2 | Duplicate response types | ~80 | Medium | contract tests + API check |
| B3 | Shared profile form | ~400 | Medium | build + manual test |
| B4 | Shared event form | ~150 | Medium | build + manual test |
| B5 | BaseListViewModel | ~150 | Medium | build + manual test |
| B6 | Merge event files | ~100 | Low | build/test/clippy + metrics |
| B7 | Merge auth/upload files | ~60 | Low | build/test/clippy |
| B8 | Simplify chat bubbles | ~80 | Low | build + manual test |
| B9 | Simplify AppNavigation | ~50 | Low | build + manual test |
| C1-C4 | Readability (no line target) | ~0 | Low | build/test/clippy/lint |
| | **Total (with vendor)** | **~12,800** | | |
| | **Total (without vendor)** | **~1,800** | | |

## Execution order

**Phase 1 — Investigate vendor removal (A1):**
Check why each patch exists. If removable, this alone exceeds the 5k target.

**Phase 2 — Dead code deletion (A2–A5), one PR:**
Low risk, no behavior change, ~600 lines. Run all quality gates.

**Phase 3 — Legacy auth cleanup (B1), one PR:**
Requires confirming mobile doesn't use legacy paths. ~150 lines.

**Phase 4 — Backend structural merges (B2, B6, B7, C1), 1-2 PRs:**
Type consolidation + file merges + rename. ~240 lines. Run contract tests.

**Phase 5 — Mobile component extraction (B3, B4, B5), 2-3 PRs:**
Largest mobile wins. ~700 lines. Each PR covers one extraction, tested manually.

**Phase 6 — Mobile polish (B8, B9, C2–C4), 1-2 PRs:**
Smaller wins + readability. ~130 lines.

**Every PR must pass:** `cargo fmt/clippy/test`, `rust-code-analysis.sh`, `./gradlew ktlintCheck detekt`, `./gradlew :composeApp:assembleDebug`.
