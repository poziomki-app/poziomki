# Success Criteria Checklist

Quality gates that must pass before release.

## Code Quality

### Linting (oxlint)
- [ ] 0 oxlint warnings
- [ ] All rules from `.oxlintrc.json` passing
- [ ] CI blocks merge on any violation

### TypeScript (strict mode)
- [ ] 0 TypeScript errors
- [ ] All `strict` flags enabled in `tsconfig.json`
- [ ] No `any` types anywhere
- [ ] No `as Type` assertions anywhere
- [ ] No `!` non-null assertions anywhere
- [ ] No `@ts-ignore` / `@ts-expect-error` / `@ts-nocheck`
- [ ] No `// eslint-disable` comments

### Code Structure
- [ ] All functions < 50 LOC
- [ ] Cyclomatic complexity < 10 per function
- [ ] No deeply nested callbacks (max 3 levels)

## Test-Driven Development

- [ ] Tests written BEFORE implementation for new code
- [ ] Characterization tests for existing code before refactoring
- [ ] 80%+ overall test coverage
- [ ] 90%+ API route coverage
- [ ] 85%+ service layer coverage
- [ ] 80%+ hook coverage
- [ ] CI fails if coverage drops
- [ ] All tests pass before merge

## Security

- [ ] Hardware-backed E2E encryption verified
- [ ] Private keys never leave secure hardware
- [ ] ECDH key agreement working
- [ ] AES-GCM encryption/decryption working
- [ ] Key exchange flow tested
- [ ] All routes require auth (except public)
- [ ] All mutations use transactions
- [ ] No secrets in codebase
- [ ] No secrets in logs
- [ ] File access validated (ownership check)
- [ ] All containers hardened
- [ ] All containers with `no-new-privileges`
- [ ] All containers with `cap_drop: ALL`

## Performance

- [ ] App startup < 200ms
- [ ] Message list 60fps scroll
- [ ] Discovery feed 60fps scroll
- [ ] Events list 60fps scroll
- [ ] APK size < 20MB
- [ ] Memory usage < 100MB
- [ ] No memory leaks detected
- [ ] Battery usage reasonable

## Accessibility

- [ ] Screen reader compatible (TalkBack/VoiceOver tested)
- [ ] All interactive elements have labels
- [ ] Touch targets ≥ 44x44
- [ ] Dark mode contrast WCAG AA (4.5:1 minimum)
- [ ] Reduced motion support (`prefers-reduced-motion`)
- [ ] High contrast mode support (`prefers-contrast: high`)
- [ ] Focus indicators visible
- [ ] Keyboard navigation works (where applicable)

## Local-First

- [ ] Chat works fully offline
- [ ] Messages queue when offline
- [ ] Messages sync when back online
- [ ] Sync status visible to user
- [ ] No data loss on network failures
- [ ] Sub-50ms message list render
- [ ] Conflict resolution works (tested with concurrent edits)

## Infrastructure

- [ ] All containers using hardened images (Chainguard where available)
- [ ] All containers with security options applied
- [ ] SeaweedFS serving uploads
- [ ] Dragonfly rate limiting working
- [ ] Electric SQL syncing chat data
- [ ] All services health-checked
- [ ] Data migration from MinIO complete
- [ ] No data loss during migration

## Encryption

- [ ] Keys generated in hardware (Secure Enclave / StrongBox)
- [ ] Private keys never leave secure hardware
- [ ] ECDH key agreement computed in hardware
- [ ] Symmetric keys derived correctly
- [ ] AES-GCM encryption produces valid ciphertext
- [ ] AES-GCM decryption recovers plaintext
- [ ] Cross-device encryption works
- [ ] Server cannot decrypt messages (verified)

## Feature Parity

See [feature-parity.md](./feature-parity.md) for complete list.

**Auth:**
- [ ] Email + OTP login works
- [ ] Session persists across app restart
- [ ] Account deletion removes all data
- [ ] Data export downloads complete JSON

**Profiles:**
- [ ] Discovery feed loads and scrolls
- [ ] Profile detail shows all fields
- [ ] Edit profile saves changes
- [ ] Multiple photos upload and display
- [ ] Tags selection works
- [ ] Bookmarks save and display
- [ ] Matching shows compatible profiles first

**Events:**
- [ ] Events list with filters works
- [ ] Event detail shows all info
- [ ] Create event with all fields
- [ ] Edit event updates correctly
- [ ] Attend/leave toggles status
- [ ] Attendee list displays
- [ ] Event chat accessible to attendees only

**Chat (All Features):**
- [ ] Conversations list with unread counts
- [ ] Messages load with pagination
- [ ] Send text message (encrypted)
- [ ] Send image attachment
- [ ] Reactions add/remove/toggle
- [ ] Reaction counts display
- [ ] Reaction breakdown shows who reacted
- [ ] Reply to message works
- [ ] Edit own message
- [ ] Delete own message
- [ ] Typing indicator shows/hides
- [ ] Read receipts update
- [ ] Mentions highlight and link
- [ ] Context menu shows all options
- [ ] Offline: messages queue and sync
- [ ] Offline: read old messages

**Visual Parity:**
- [ ] Colors match exactly
- [ ] Gradients match exactly
- [ ] Typography matches exactly
- [ ] Spacing matches exactly
- [ ] Animations feel the same

## Metrics Achievement

| Metric | Target | Actual | Pass |
|--------|--------|--------|------|
| Screens | 12 | | |
| Hooks | 15 | | |
| Components | 35 | | |
| Mobile LOC | ~8,000 | | |
| API LOC | ~4,500 | | |
| Dependencies | ~150 | | |
| APK Size | ~18 MB | | |
| oxlint warnings | 0 | | |
| TypeScript errors | 0 | | |
| Type assertions | 0 | | |
| Test coverage | 80%+ | | |

## Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Developer | | | |
| Security Review | | | |
| QA | | | |
| Product Owner | | | |
