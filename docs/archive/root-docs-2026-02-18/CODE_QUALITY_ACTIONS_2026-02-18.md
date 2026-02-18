# Code Quality and Surface Reduction Plan

Date: 2026-02-18

## How this was researched

I validated current state by running project quality gates and scanning for hotspots/duplication.

- Backend: `cargo fmt --all -- --check` -> PASS
- Backend: `cargo clippy --workspace --all-targets --all-features -- -D warnings` -> PASS
- Backend metrics: `backend/scripts/rust-code-analysis.sh` -> FAIL (11 files over thresholds)
- Mobile: `./gradlew ktlintCheck detekt` -> FAIL at ktlint (`ProfileEditScreen.kt:3`, `ProfileEditScreen.kt:664`)
- Mobile: `./gradlew detekt` -> SUCCESS but both modules report `NO-SOURCE` (detekt effectively not scanning app code)
- Backend tests: `cargo test --all-features` -> unit tests pass, integration tests fail with `SqlxError(PoolTimedOut)` in `backend/tests/requests/migration_contract.rs`

Additional structural findings:

- Backend hotspots: `matching.rs` (832), `state_types.rs` (429), `search.rs` (418), `matrix_support.rs` (394), `auth_helpers.rs` (375), `profiles_mutations.rs` (352), `events_mutations.rs` (347)
- Mobile hotspots: `ProfileEditScreen.kt` (1003), `ChatContent.kt` (806), `EventCreateScreen.kt` (731), `ChatViewModel.kt` (565), `AppNavigation.kt` (491)
- Boilerplate mapping pattern appears 68 times: `.map_err(|e| loco_rs::Error::Any(e.into()))`
- `TagScope` string conversion duplicated in 3 files: `catalog.rs`, `matching.rs`, `profiles.rs`
- `Profile` and `ProfileWithTags` are near-duplicate API models in `mobile/shared/src/commonMain/kotlin/com/poziomki/app/api/Models.kt`
- Mobile production sources have no test sources (`composeApp/src/**/test`, `shared/src/**/test` not present)
- iOS picker remains TODO-only in `mobile/composeApp/src/iosMain/kotlin/com/poziomki/app/util/ImagePicker.ios.kt`
- Mobile GitHub workflows are template-gated and effectively disabled (`if: github.event.repository.name == 'KMP-App-Template'`)

## Priority Backlog (Actionable)

## P0: Fix broken quality gates and blind spots first

1. Fix current ktlint violations in `mobile/composeApp/src/commonMain/kotlin/com/poziomki/app/ui/screen/profile/ProfileEditScreen.kt`.
- Why: quality gate currently red.
- Done when: `./gradlew ktlintCheck` passes.

2. Make detekt actually scan Kotlin source sets (not `NO-SOURCE`) in `mobile/build.gradle.kts`.
- Why: static analysis is configured but ineffective.
- Action: configure detekt source includes for KMP (`composeApp/src`, `shared/src`) and wire report outputs in CI.
- Done when: `./gradlew detekt` reports analyzed files and findings count.

3. Stabilize backend integration tests failing with DB pool timeouts in `backend/tests/requests/migration_contract.rs`.
- Why: tests are not reliable locally; signal quality is weak.
- Action: provide explicit test DB bootstrap and pool sizing for loco test harness; document required env for local run.
- Done when: `cargo test --all-features --test mod` passes in a clean local env.

4. Enable real mobile CI checks in `mobile/.github/workflows/build-android.yml` and `mobile/.github/workflows/build-ios.yml`.
- Why: current workflows are template-guarded and skipped.
- Action: remove template-only `if`, then enforce `ktlintCheck detekt` and at least Android build.
- Done when: PRs run mobile lint/static checks automatically.

## P1: Reduce backend complexity to pass rust-code-analysis

5. Split `backend/src/controllers/migration_api/matching.rs` by responsibility.
- Suggested slices: scoring math, DB loaders, HTTP handlers, and tests.
- Why: largest file and metrics offender.
- Done when: file-level MI/complexity drops under script thresholds.

6. Split `backend/src/search.rs` into `search_documents.rs`, `search_indexing.rs`, `search_query.rs` (or similar).
- Why: low MI file and mixed responsibilities.
- Note: move test-only serialization checks to dedicated test module to shrink prod unit.
- Done when: `backend/scripts/rust-code-analysis.sh` no longer flags this unit.

7. Refactor `backend/src/controllers/migration_api/profiles_mutations.rs` and `backend/src/controllers/migration_api/events_mutations.rs` into shared validation/update helpers.
- Why: both are metrics failures and contain similar update/tag-sync flow.
- Done when: both files clear thresholds with no API shape changes.

8. Break up `backend/src/controllers/migration_api/auth_helpers.rs` and `backend/src/controllers/migration_api/matrix_support.rs`.
- Why: high exits / arg-count / low maintainability.
- Action: isolate pure helpers (env/config parsing, payload building, response parsing) from IO code.
- Done when: rust metrics gate passes for both.

9. Simplify repetitive index migration in `backend/migration/src/m20250217_000010_add_indexes.rs`.
- Why: cyclomatic threshold failure caused by repetitive index creation calls.
- Action: use a small helper and declarative index list.
- Done when: migration file clears cyclomatic threshold.

## P2: Remove repeated low-quality code and trim surface area

10. Introduce `TagScope` conversions in one place (e.g. impl on enum in `state_types.rs`) and remove duplicated `scope_from_str/str_to_scope`.
- Targets: `catalog.rs`, `matching.rs`, `profiles.rs`.
- Gain: fewer inconsistent conversions and less duplicated logic.

11. Add a `Result` extension for the repeated error conversion pattern.
- Pattern count: 68.
- Action: implement helper like `map_any_err()` and migrate high-churn files first (`matching.rs`, `auth_export_queries.rs`, `profiles.rs`).
- Gain: lower boilerplate and clearer business logic.

12. Consolidate mobile profile models.
- Targets: `mobile/shared/src/commonMain/kotlin/com/poziomki/app/api/Models.kt`, `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/mapper/ProfileMapper.kt`, `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/ProfileRepository.kt`.
- Action: merge `Profile` and `ProfileWithTags` into one model with optional/default `tags`.
- Gain: removes duplicate conversion code and reduces API surface.

13. Extract reusable offline mutation helper in repositories.
- Targets: `EventRepository.kt`, `ProfileRepository.kt`, `SettingsRepository.kt`.
- Why: repeated `isOnline + pendingOps + optimistic update` flow.
- Gain: meaningful line reduction and fewer divergent edge-case behaviors.

14. Remove unsafe null assertions in production UI paths.
- Targets: `ProfileEditScreen.kt` (`gradientStart!!`, `gradientEnd!!`), `ExploreScreen.kt` (`searchResults!!`).
- Why: avoid crash-prone code; raise confidence for future refactors.

15. Either implement or explicitly disable iOS picker actions.
- Target: `mobile/composeApp/src/iosMain/kotlin/com/poziomki/app/util/ImagePicker.ios.kt`.
- Why: current TODO stubs are effectively dead UX paths.

## P3: Architecture hardening after cleanup

16. Replace hand-maintained DTO drift with contract-driven generation.
- Targets: backend DTOs in `state_types.rs` and mobile API models in `Models.kt`.
- Why: current manual duplication causes surface growth and inconsistency risk.

17. Add lightweight module budget rules.
- Example rule: warn on files > 400 LOC in backend controllers and > 500 LOC in Compose screens.
- Why: prevents hotspot regrowth after cleanup.

## Suggested execution order

1. P0.1 -> P0.4 (restore signal from gates/CI).
2. P1.5 + P1.6 (largest metric wins first).
3. P1.7 + P1.8 + P1.9 (finish rust metrics gate).
4. P2.10 -> P2.15 (duplication and low-quality code removal).
5. P3.16 -> P3.17 (longer-term guardrails).

## Definition of done for this initiative

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `backend/scripts/rust-code-analysis.sh` passes.
- `./gradlew ktlintCheck detekt` passes with detekt analyzing real source files.
- Backend integration tests are stable (no pool-timeout failures in normal local setup).
- No remaining `!!` in production UI code without explicit guard/justification.
