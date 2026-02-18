# Code Simplification and Quality Strategy

Updated: 2026-02-17

This is the single source of truth for simplification and maintainability work.
It consolidates the useful points from prior `CODE_SIMPLIFICATION*.md` documents and removes stale findings.

## North Star

Reduce code surface area and complexity while preserving behavior and API contracts.

## Non-Negotiable Quality Gates

Backend:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `backend/scripts/rust-code-analysis.sh`

Mobile:
- `./gradlew ktlintCheck detekt` (run from `mobile/`)

Do not bypass these checks in CI.

## Current Baseline (Validated)

### Backend
- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: PASS (first-party code)
- `backend/scripts/rust-code-analysis.sh`: FAIL

Current rust-code-analysis failures are concentrated in:
- `backend/src/controllers/migration_api/matching.rs`
- `backend/src/controllers/migration_api/search_api.rs`
- `backend/src/controllers/migration_api/state_uploads.rs`
- `backend/src/controllers/migration_api/profiles_mutations.rs`
- `backend/src/controllers/migration_api/events_mutations.rs`
- `backend/src/controllers/migration_api/matrix_support.rs`
- `backend/src/controllers/migration_api/auth_helpers.rs`
- `backend/src/search.rs`
- `backend/src/tasks/seed_search.rs`
- `backend/migration/src/m20250217_000010_add_indexes.rs`
- `backend/src/controllers/migration_api/matrix.rs` (arg-count threshold)

Large concentration files in `migration_api` (LOC):
- `matching.rs` (832)
- `state_types.rs` (429)
- `matrix_support.rs` (394)
- `auth_helpers.rs` (375)
- `profiles_mutations.rs` (352)
- `events_mutations.rs` (347)

### Mobile
- Not re-run in this pass; strategy keeps only validated points from repository inspection.
- Very large UI/viewmodel files remain:
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/profile/ProfileEditScreen.kt` (1003)
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/event/EventCreateScreen.kt` (731)
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/chat/ChatViewModel.kt` (565)
  - `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/navigation/AppNavigation.kt` (491)

### Validation corrections (stale findings removed)
- Legacy backend scaffold (`controllers/auth.rs`, `views/`, `mailers/`) is already removed.
- `SessionManager.kt` forward-reference issue is already fixed.
- Thin model wrappers under `backend/src/models/*.rs` are not yet dead: they host `ActiveModelBehavior` impls.

## Work Already Completed

- Empty backend stubs removed:
  - `backend/src/data/mod.rs`
  - `backend/src/initializers/mod.rs`
  - `backend/src/workers/mod.rs`
- Removed corresponding exports from `backend/src/lib.rs`.
- Deduplicated unauthorized error helper usage in auth account flow.
- Removed unused `remember_me` from sign-in payload.
- Replaced UTF-8-unsafe string slicing in OTP email logging.

## Confirmed Simplification Opportunities

1. Repetitive error mapping in backend
- Pattern `.map_err(|e| loco_rs::Error::Any(e.into()))` appears 72 times.
- Highest concentration files:
  - `matching.rs` (10)
  - `auth_export_queries.rs` (10)
  - `profiles.rs` (9)

2. Repeated `TagScope` conversion logic
- Duplicated match blocks appear in:
  - `profiles.rs`
  - `matching.rs`
  - `events_view.rs`
  - `catalog.rs`

3. Search indexing wrapper duplication
- `search.rs` repeats near-identical `index_*` and `delete_*` wrapper functions.

4. Oversized mixed-responsibility handlers
- Several `migration_api` files mix validation, query orchestration, mapping, and side effects in one unit.

5. Mobile null-safety and placeholder work
- Remaining `!!` usage in current code includes:
  - `ProfileEditScreen.kt`
  - `ExploreScreen.kt`
- iOS picker TODOs still present:
  - `mobile/composeApp/src/iosMain/kotlin/com/poziomki/app/util/ImagePicker.ios.kt`

## Strategic Roadmap (Actionable)

### Phase 1: Mechanical simplification (low risk, high leverage)
Target: reduce repetition with no behavior change.

Actions:
1. Introduce `ResultExt::map_any_err()` in backend and migrate occurrences incrementally.
2. Add `TagScope::from_db(&str)` and replace duplicated conversions.
3. Replace `pub(in crate::controllers::migration_api)` with `pub(super)` in `state_types.rs` where equivalent.
4. Collapse duplicated search indexing wrappers into generic helpers (`index_document`, `delete_document`).

Definition of done:
- No API response/route changes.
- All backend quality gates pass except pre-existing metric hotspots not touched in this phase.

### Phase 2: Complexity extraction in backend hotspots
Target: pass rust-code-analysis by reducing per-file complexity.

Actions:
1. `matching.rs`: split into modules:
   - candidate fetch
   - scoring/ranking
   - response mapping
2. `search_api.rs`: split request parsing + query build + response mapping.
3. `profiles_mutations.rs` and `events_mutations.rs`: isolate shared update/indexing path into support modules.
4. `state_uploads.rs`: split validation, storage interaction, and response shape.
5. `auth_helpers.rs`: break high-exit helpers into linear subfunctions.

Definition of done:
- `backend/scripts/rust-code-analysis.sh` passes.
- Route behavior validated by existing tests + focused regression tests.

### Phase 3: Contract and architecture hardening
Target: prevent drift and keep complexity from regrowing.

Actions:
1. Establish backend/mobile contract generation path (OpenAPI or schema-based DTO generation).
2. Introduce module boundaries in `migration_api` by feature area:
   - auth
   - profiles
   - events
   - matching
   - search
   - uploads/matrix
3. Add CI checks that run all quality gates for backend and mobile.

Definition of done:
- DTO duplication reduced.
- CI blocks merges on gate failures.

### Phase 4: Mobile maintainability decomposition
Target: reduce screen/viewmodel bloat and crash risk.

Actions:
1. Split `ProfileEditScreen.kt` and `EventCreateScreen.kt` into feature subcomponents + state mappers.
2. Split `AppNavigation.kt` into per-feature nav graphs.
3. Reduce `!!` by explicit state guards and typed UI states.
4. Implement iOS picker TODOs or gate unsupported actions with explicit UX fallback.

Definition of done:
- No `!!` in production UI paths without explicit justification.
- Large files reduced and responsibility boundaries clear.

## Prioritization Matrix

Do first:
1. Phase 1.1 `map_any_err` + Phase 1.2 `TagScope::from_db` (fast and broad payoff)
2. Phase 1.4 search wrapper genericization
3. Phase 2 extraction on `matching.rs` then `search_api.rs`

Do next:
4. Mutation/upload hotspot extraction
5. Mobile file decomposition and null-safety cleanup

## Guardrails During Refactor

- Keep route signatures and response contracts stable unless explicitly planned.
- Prefer additive extraction (new helper/module + call-site switch) over in-place rewrites.
- Land changes in small PR slices with one dominant concern each.
- Re-run quality gates after each slice.

## Tracking Checklist

- [ ] Phase 1 complete (mechanical simplification)
- [ ] Phase 2 complete (backend complexity gates passing)
- [ ] Phase 3 complete (contract + CI hardening)
- [ ] Phase 4 complete (mobile decomposition + null-safety)

