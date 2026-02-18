# Known Problems

Issues identified during code review and MVP gap analysis.

> **Note:** Hardcoded `localhost:5150` URLs are intentional — dev-only environment. Not a problem.

---

## CRITICAL — Play Store blockers

Must be resolved before Play Store submission.

### Privacy policy

Play Store requires a privacy policy URL in the store listing and ideally linked in-app. No policy exists yet.

### App signing config

No release signing keystore or signing config in Gradle. Needed to produce a signed release bundle for upload.

### Account deletion flow

Play Store policy (enforced since 2024) requires apps with accounts to offer in-app account deletion. Backend has `DELETE /api/v1/auth/account` but mobile `PrivacyScreen` has `/* TODO: delete account */` stub.

### Data export

Related to account deletion — users should be able to export their data. Backend has `GET /api/v1/auth/export` but mobile is stubbed.

### Settings not persisted to backend

`SettingsRepository` queues changes locally but never calls `/api/v1/settings` PATCH. Privacy toggles (show age, show program, discoverable) are UI-only.

### Email domain whitelist hardcoded

Magic link auth is locked to `@example.com` and `@gmail.com` in `is_allowed_email_domain()`. Needs to be configurable or scoped to university domains for launch.

### iOS chat unavailable (Matrix client is Noop)

iOS DI still binds `MatrixClient` to `NoopMatrixClient`, so chat is effectively unavailable on iOS even though Android chat is functional.

- `mobile/shared/src/iosMain/kotlin/com/poziomki/app/di/PlatformModule.ios.kt:20`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/chat/matrix/api/NoopMatrixClient.kt:7`

---

## HIGH — ktlint violations (shared module)

4 violations in 3 files. All auto-fixable (`./gradlew ktlintFormat`).

- `shared/.../chat/matrix/impl/RustMatrixClient.kt:483` — `when-entry-bracing`: inconsistent braces
- `shared/.../chat/matrix/impl/RustTimeline.kt:231` — `when-entry-bracing`: inconsistent braces
- `shared/.../chat/matrix/impl/RustTimeline.kt:232` — `blank-line-between-when-conditions`: missing blank line
- `shared/.../chat/matrix/api/NoopMatrixClient.kt:23` — `function-signature`: body expression fits on same line

## HIGH — Event chat room mapping endpoint is not implemented in backend

Mobile currently creates event rooms client-side, but backend route exists and returns `NOT_IMPLEMENTED`, which risks inconsistent event->room mapping across devices/clients.

- `backend/src/controllers/migration_api/mod.rs:197` — `/api/v1/matrix/events/{eventId}/room`
- `backend/src/controllers/migration_api/mod.rs:104` — `not_implemented(...)`
- `mobile/shared/src/commonMain/kotlin/com/poziomki/app/data/repository/EventRepository.kt:264` — `ensureEventRoom(...)` local creation path

## HIGH — `runBlocking` on main/UI thread (ANR risk)

Blocks the UI thread during composition, causing jank on cold start.

- `composeApp/.../App.kt:37` — `runBlocking { sessionManager.isLoggedIn.first() }` as `collectAsState` initial value
- `composeApp/.../App.kt:44-45` — two `runBlocking` calls inside `remember {}` for start destination

Consider using a splash screen state with `LaunchedEffect` or `produceState` instead.

## HIGH — Backend test suite red in current environment

`cargo test -q` fails all backend tests due to PostgreSQL auth/config mismatch (`ident auth failed for user "loco"`), which blocks reliable pre-release verification.

- `backend/tests/models/users.rs:95` — first failure path while booting test app
- `backend/tests/requests/migration_contract.rs:300` — request test failure path

## MEDIUM — Events lack visibility levels

All events are community-only (require auth). No way to make events public (for a webpage) or private (invite-only).

Needs:
- `visibility` column on events table (`public`, `community`, `private`)
- Public API endpoint (`GET /api/v1/public/events`) — no auth, returns only public events, consumable by a website
- Matrix room join rules aligned to visibility (`public` → peekable, `community` → knock, `private` → invite)
- Existing event list queries filtered by visibility + user permissions

## MEDIUM — Swallowed exceptions (no logging)

Catch blocks discard exceptions without logging, hiding failures silently.

- `shared/.../data/sync/SyncEngine.kt:118` — `catch (_: Exception)` in sync loop, only manages retry state
- `shared/.../api/ApiClient.kt:136,177` — `catch (_: Exception)` when parsing error responses, returns generic error
- `composeApp/.../ui/navigation/AppNavigation.kt:192,206,221` — `catch (_: Exception)` in nav back stack operations
- `composeApp/.../util/ImagePicker.android.kt:80` — `catch (_: Exception)` in image compression, returns null

## MEDIUM — Unimplemented features (TODOs)

- `composeApp/.../ui/screen/profile/PrivacyScreen.kt:125` — `/* TODO: export data */`
- `composeApp/.../ui/screen/profile/PrivacyScreen.kt:185` — `/* TODO: delete account */`
- `composeApp/src/iosMain/.../util/ImagePicker.ios.kt:7,10` — iOS image picker stubs (both single and multi)

## LOW — Non-null assertions (`!!`) after null checks

These are safe due to preceding `if` checks / `when` branches, but could use Kotlin smart-cast (`let`, destructuring, or early return) for idiomatic safety.

- `composeApp/.../ui/screen/profile/ProfileViewScreen.kt:58` — `state.profile!!`
- `composeApp/.../ui/screen/main/ProfileScreen.kt:88` — `state.profile!!`
- `composeApp/.../ui/screen/event/EventDetailScreen.kt:77` — `state.event!!`
- `composeApp/.../ui/screen/onboarding/ProfileSetupScreen.kt:111,193` — `state.error!!`, `state.selectedAvatar!!`
- `composeApp/.../ui/screen/auth/LoginScreen.kt:80` — `uiState.error!!`
- `composeApp/.../ui/screen/auth/RegisterScreen.kt:78` — `uiState.error!!`
- `composeApp/.../ui/screen/auth/VerifyScreen.kt:111` — `uiState.error!!`
