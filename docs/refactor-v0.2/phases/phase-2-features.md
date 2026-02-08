# Phase 2: Features + Infrastructure (Weeks 3-4)

## Overview

Build core features (profiles, events) and set up local-first infrastructure.

## Week 3: Profiles + Events

### Goals
- Discovery feed with infinite scroll
- Profile detail and edit
- Events list with filters
- Event detail, create, edit

### Tasks

- [ ] **Tests First**
  - [ ] Write API tests for profile endpoints
  - [ ] Write API tests for event endpoints
  - [ ] Write characterization tests against existing API
  - [ ] Verify all contracts match

- [ ] **Profile Components**
  - [ ] `ProfileCard` - card in discovery feed
  - [ ] `ProfileDetail` - full profile view (modal)
  - [ ] `ProfileEdit` - edit form
  - [ ] `TagSelector` - hierarchical tag picker
  - [ ] `Avatar` - image with fallback

- [ ] **Discovery Feed**
  - [ ] Implement `useProfiles()` hook
  - [ ] Infinite scroll with `useInfiniteList()`
  - [ ] Pull-to-refresh
  - [ ] Profile matching (sort by shared tags)
  - [ ] Bookmark functionality

- [ ] **Event Components**
  - [ ] `EventCard` - card in events list
  - [ ] `EventDetail` - full event view (modal)
  - [ ] `EventForm` - create/edit form
  - [ ] `AttendeeList` - list of attendees
  - [ ] `DateTimePicker` - date/time selection

- [ ] **Events List**
  - [ ] Implement `useEvents()` hook
  - [ ] Time filters (today, this week, etc.)
  - [ ] Event attendance (join/leave)
  - [ ] Event cover image upload

- [ ] **Hook Tests**
  - [ ] Write tests for `useProfiles()`
  - [ ] Write tests for `useEvents()`
  - [ ] Write tests for `useDiscovery()`

- [ ] **Visual Consistency**
  - [ ] All screens use gradient backgrounds
  - [ ] All cards use gradient elevation
  - [ ] Consistent spacing and typography

### Deliverables
- [ ] Discovery feed loads and scrolls smoothly
- [ ] Can view/edit own profile
- [ ] Events list with working filters
- [ ] Can create/edit events
- [ ] Can attend/leave events
- [ ] All hook tests passing

## Week 4: Infrastructure + Local-First Setup

### Goals
- SeaweedFS replacing MinIO
- Dragonfly for caching
- Electric SQL syncing chat data
- TanStack DB configured

### Tasks

- [ ] **MinIO → SeaweedFS Migration**
  - [ ] Deploy SeaweedFS container
  - [ ] Configure S3 credentials
  - [ ] Mirror data from MinIO
  - [ ] Update API environment variables
  - [ ] Verify uploads work

- [ ] **Dragonfly Setup**
  - [ ] Deploy Dragonfly container (Chainguard)
  - [ ] Configure rate limiting
  - [ ] Configure session cache
  - [ ] Test pub/sub for typing indicators

- [ ] **Electric SQL Setup**
  - [ ] Deploy Electric SQL container
  - [ ] Enable Postgres logical replication
  - [ ] Configure AUTH_JWT_KEY
  - [ ] Test basic sync

- [ ] **TanStack DB Setup**
  - [ ] Define collections (messages, conversations, reactions)
  - [ ] Configure Electric sync adapter
  - [ ] Define sync shapes (user's conversations only)
  - [ ] Test local write → sync

- [ ] **Integration Tests**
  - [ ] Test offline message creation
  - [ ] Test sync on reconnect
  - [ ] Test conflict resolution
  - [ ] Test across multiple devices

- [ ] **Hardening**
  - [ ] All containers with `no-new-privileges`
  - [ ] All containers with `cap_drop: ALL`
  - [ ] Health checks on all services
  - [ ] Verify no exposed ports except necessary

### Deliverables
- [ ] SeaweedFS serving uploads
- [ ] Dragonfly handling rate limits
- [ ] Electric SQL syncing data
- [ ] TanStack DB working offline
- [ ] All containers hardened
- [ ] Integration tests passing

## Dependencies

**From Phase 1:**
- Working auth flow
- Design system components

**External:**
- MinIO data to migrate
- Postgres with logical replication enabled

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Data loss during MinIO migration | Full backup before migration, mirror (not move) |
| Electric SQL connection issues | Start simple, add complexity gradually |
| TanStack DB sync conflicts | Use built-in CRDT resolution |

## Success Criteria

- [ ] Profile discovery works (60fps scroll)
- [ ] Events fully functional
- [ ] SeaweedFS serving all uploads
- [ ] Dragonfly rate limiting working
- [ ] Electric SQL sync verified
- [ ] TanStack DB offline mode works
- [ ] All tests passing
- [ ] 0 lint/type errors
