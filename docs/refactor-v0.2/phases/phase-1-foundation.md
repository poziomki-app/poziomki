# Phase 1: Foundation (Weeks 1-2)

## Overview

Set up the LynxJS project, design system, and authentication. This phase establishes the base for all other work.

## Week 1: Spike + Design System

### Goals
- Working LynxJS app on real Android device
- Dark-only design system with gradients
- Type-safe API client (Eden)
- Basic navigation

### Tasks

- [ ] **Project Setup**
  - [ ] Initialize LynxJS project with Lynx Community CLI
  - [ ] Configure TypeScript strict mode from day 1
  - [ ] Configure oxlint with all rules
  - [ ] Set up bun workspace structure
  - [ ] Copy environment files from old repo for reference

- [ ] **Design System**
  - [ ] Create `theme.css` with all CSS variables
  - [ ] Implement `Screen` component (gradient background)
  - [ ] Implement `Card` component (gradient elevation)
  - [ ] Implement `Button` component (variants)
  - [ ] Implement `Input` component
  - [ ] Implement `Text` component (typography)
  - [ ] **Write component tests for Screen, Card, Button**

- [ ] **API Client**
  - [ ] Install Eden client
  - [ ] Configure API base URL
  - [ ] Verify type inference from existing API
  - [ ] Create API wrapper with auth header injection

- [ ] **Navigation**
  - [ ] Set up TanStack Router (file-based)
  - [ ] Create tab layout structure
  - [ ] Create placeholder screens for all 12 routes
  - [ ] Implement TabBar component with gradients

- [ ] **Device Testing**
  - [ ] Build APK
  - [ ] Test on real Android device
  - [ ] Verify gradients render correctly
  - [ ] Verify navigation works

### Deliverables
- [ ] APK runs on device with placeholder screens
- [ ] Design system components documented
- [ ] 0 oxlint warnings
- [ ] 0 TypeScript errors

## Week 2: Auth + Secure Storage

### Goals
- Working login flow
- Hardware-backed secure storage
- Encryption keypair generated
- Onboarding screen

### Tasks

- [ ] **Tests First**
  - [ ] Write API tests for auth endpoints (login, verify, logout)
  - [ ] Write service tests for session management
  - [ ] Run tests against existing API to verify contracts

- [ ] **Native Modules**
  - [ ] Implement SecureStorage native module (Android)
  - [ ] Implement SecureStorage native module (iOS)
  - [ ] Test secure storage read/write

- [ ] **Login Flow**
  - [ ] Build login screen (email input)
  - [ ] Build OTP verification (inline, not separate screen)
  - [ ] Implement session token storage (SecureStorage)
  - [ ] Implement auto-login on app start
  - [ ] Implement logout

- [ ] **Encryption Setup**
  - [ ] Generate P-256 keypair on first login (hardware-backed)
  - [ ] Store keypair reference in SecureStorage
  - [ ] Upload public key to server
  - [ ] **Write crypto tests for key generation**

- [ ] **Onboarding**
  - [ ] Build single combined onboarding screen
  - [ ] Multi-step form: basic info → profile → interests
  - [ ] Progress indicator
  - [ ] Profile photo upload placeholder

- [ ] **Hook Tests**
  - [ ] Write tests for `useAuth()` hook
  - [ ] Test login/logout flows
  - [ ] Test session persistence

### Deliverables
- [ ] Can login, stay logged in, logout
- [ ] Encryption keypair generated on device
- [ ] Onboarding flow works
- [ ] Auth hook tests passing
- [ ] 0 oxlint warnings, 0 TypeScript errors

## Dependencies

**From old repo (reference only):**
- Auth API contracts (verify endpoints match)
- Color values from `unistyles.ts`

**External:**
- Existing API running (unchanged)
- Test user accounts

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| LynxJS native module issues | Start with simple modules, escalate early |
| Secure Enclave not available (old devices) | Fallback to software keystore |
| Eden type inference broken | Test early in Week 1 |

## Success Criteria

- [ ] APK installs and runs
- [ ] Login → OTP → Onboarding → Home flow works
- [ ] Session persists across app restart
- [ ] Encryption keypair stored in hardware
- [ ] All tests passing
- [ ] 0 lint/type errors
