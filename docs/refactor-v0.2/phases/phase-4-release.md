# Phase 4: Polish + Release (Weeks 7-8)

## Overview

Complete remaining features, audit quality, and prepare for release.

## Week 7: Polish + Native Modules

### Goals
- Camera/image picker
- Complete reactions
- Haptics and polish
- Remaining component tests

### Tasks

- [ ] **Image Handling**
  - [ ] Implement camera picker native module
  - [ ] Implement gallery picker native module
  - [ ] Image compression before upload
  - [ ] Upload progress indicator
  - [ ] `PhotoPicker` component
  - [ ] `ImageGallery` viewer (pinch to zoom)

- [ ] **Chat Images**
  - [ ] Send image in chat (encrypted)
  - [ ] Image thumbnails in messages
  - [ ] Tap to view full image

- [ ] **Reactions Polish**
  - [ ] `ReactionPicker` inline component
  - [ ] Smooth animations
  - [ ] Haptic feedback on react

- [ ] **Haptics & Feedback**
  - [ ] Button press haptics
  - [ ] Tab switch haptics
  - [ ] Pull-to-refresh haptics
  - [ ] Message send haptic

- [ ] **Remaining Components**
  - [ ] `EmptyState` for empty lists
  - [ ] `ErrorBoundary` for crash handling
  - [ ] Loading states for all screens
  - [ ] Error states for all screens

- [ ] **Component Tests**
  - [ ] Write remaining component tests
  - [ ] **Coverage check: 80%+ overall**
  - [ ] Visual regression tests (optional)

- [ ] **Performance Optimization**
  - [ ] Profile scroll performance (60fps)
  - [ ] Reduce re-renders
  - [ ] Optimize image loading
  - [ ] Memory profiling

### Deliverables
- [ ] Image upload/view works everywhere
- [ ] Haptics on all interactions
- [ ] All edge cases handled (empty, error, loading)
- [ ] 80%+ test coverage overall

## Week 8: Testing + Release

### Goals
- Full audit (accessibility, security, performance)
- OTA update setup
- App store preparation
- Final verification

### Tasks

- [ ] **Accessibility Audit**
  - [ ] Screen reader testing (TalkBack/VoiceOver)
  - [ ] Verify all touch targets ≥ 44x44
  - [ ] Verify contrast ratios (WCAG AA)
  - [ ] Test reduced motion preference
  - [ ] Test high contrast mode

- [ ] **Security Review**
  - [ ] Verify hardware-backed encryption
  - [ ] Verify no secrets in codebase
  - [ ] Verify all routes require auth
  - [ ] Verify file access validated
  - [ ] Penetration test (basic)

- [ ] **Performance Profiling**
  - [ ] App startup time < 200ms
  - [ ] Message list 60fps scroll
  - [ ] Memory usage < 100MB
  - [ ] APK size < 20MB
  - [ ] Battery usage baseline

- [ ] **Final Code Quality**
  - [ ] **0 oxlint warnings verified**
  - [ ] **0 TypeScript errors verified**
  - [ ] **80%+ coverage verified**
  - [ ] No `any`, `as`, `!` anywhere
  - [ ] All TODOs resolved

- [ ] **OTA Updates**
  - [ ] Set up self-hosted bundle server
  - [ ] Configure app to check for updates
  - [ ] Test update flow
  - [ ] Rollback capability

- [ ] **Release Preparation**
  - [ ] App icons (all sizes)
  - [ ] Splash screen
  - [ ] Store screenshots
  - [ ] Store description
  - [ ] Privacy policy URL
  - [ ] Version number set

- [ ] **Feature Parity Verification**
  - [ ] Run through entire feature checklist
  - [ ] Side-by-side comparison with old app
  - [ ] User acceptance testing

- [ ] **Build & Deploy**
  - [ ] Production APK build
  - [ ] Sign APK
  - [ ] Internal testing track
  - [ ] Beta release
  - [ ] Production release plan

### Deliverables
- [ ] All audits pass
- [ ] All quality gates pass
- [ ] OTA updates working
- [ ] APK ready for release
- [ ] Store listing complete

## Dependencies

**From Phase 3:**
- Chat fully working
- Encryption complete

**External:**
- App signing key
- Store developer account

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Accessibility issues found late | Build accessible from start (we did) |
| Performance regression | Profile continuously, not just at end |
| Store rejection | Follow guidelines from start |

## Release Checklist

### Before Internal Testing
- [ ] All features working
- [ ] No crashes in testing
- [ ] All tests passing
- [ ] Build signed

### Before Beta Release
- [ ] Internal feedback addressed
- [ ] Performance acceptable
- [ ] No critical bugs

### Before Production Release
- [ ] Beta feedback addressed
- [ ] Legal review (privacy policy)
- [ ] Support documentation ready
- [ ] Monitoring/analytics ready
- [ ] Rollback plan documented

## Success Criteria

- [ ] App startup < 200ms
- [ ] Message list 60fps
- [ ] APK size < 20MB
- [ ] Memory < 100MB
- [ ] WCAG AA contrast
- [ ] Screen reader works
- [ ] 80%+ test coverage
- [ ] 0 lint/type errors
- [ ] All features from checklist work
- [ ] Store listing approved
