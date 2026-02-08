# Phase 3: Chat + Encryption (Weeks 5-6)

## Overview

Build the full chat system with local-first architecture and end-to-end encryption.

## Week 5: Chat Core (Local-First)

### Goals
- Conversations list
- Message list (local-first)
- Send/receive messages
- Sync status indicator

### Tasks

- [ ] **Tests First (Critical)**
  - [ ] Write comprehensive tests for `useChat()` hook
  - [ ] Test all features: send, edit, delete, reactions, typing, read receipts
  - [ ] Test offline scenarios
  - [ ] Test sync scenarios
  - [ ] **Coverage check: chat module 90%+**

- [ ] **Chat Components**
  - [ ] `ChatList` - conversations list
  - [ ] `ChatItem` - single conversation row (with unread badge)
  - [ ] `ChatScreen` - main chat view container
  - [ ] `MessageList` - virtualized message list
  - [ ] `MessageBubble` - single message (mine vs theirs)
  - [ ] `ChatInput` - message composer
  - [ ] `SyncStatus` - offline/pending indicator

- [ ] **useChat() Hook**
  - [ ] Implement full `useChat()` hook (see 03-local-first-chat.md)
  - [ ] Live queries for messages
  - [ ] Live queries for reactions
  - [ ] Live queries for typing indicators
  - [ ] Live queries for read receipts
  - [ ] All mutations (send, edit, delete, react, etc.)

- [ ] **Conversations List**
  - [ ] Implement `useChats()` hook
  - [ ] Sort by last message
  - [ ] Unread count badges
  - [ ] Tap to open chat

- [ ] **Message Features**
  - [ ] Send text messages
  - [ ] Message timestamps
  - [ ] Delivery status (sent, synced)
  - [ ] Pagination (load more on scroll up)

- [ ] **Sync Features**
  - [ ] Offline indicator
  - [ ] Pending changes count
  - [ ] Auto-sync when online
  - [ ] Conflict resolution (automatic)

### Deliverables
- [ ] Chat list shows all conversations
- [ ] Messages send and appear instantly (local-first)
- [ ] Messages sync across devices
- [ ] Offline mode works fully
- [ ] All chat tests passing
- [ ] Coverage 90%+ for chat module

## Week 6: Native Crypto Module

### Goals
- Hardware-backed encryption
- Full E2E encrypted messaging
- All chat features complete

### Tasks

- [ ] **Crypto Tests First**
  - [ ] Write tests for encrypt/decrypt round-trip
  - [ ] Write tests for key generation
  - [ ] Write tests for ECDH key agreement
  - [ ] Test with known vectors

- [ ] **Android Crypto Module (Kotlin)**
  - [ ] Implement `generateKeyPair()` with StrongBox
  - [ ] Implement `getPublicKey()`
  - [ ] Implement `deleteKeyPair()`
  - [ ] Implement `deriveSharedSecret()` (ECDH)
  - [ ] Implement `encrypt()` (AES-GCM)
  - [ ] Implement `decrypt()` (AES-GCM)

- [ ] **iOS Crypto Module (Swift)**
  - [ ] Implement `generateKeyPair()` with Secure Enclave
  - [ ] Implement `getPublicKey()`
  - [ ] Implement `deleteKeyPair()`
  - [ ] Implement `deriveSharedSecret()` (ECDH)
  - [ ] Implement `encrypt()` (AES-GCM)
  - [ ] Implement `decrypt()` (AES-GCM)

- [ ] **Key Exchange**
  - [ ] Store public key on server (profile.publicKey)
  - [ ] Fetch recipient's public key
  - [ ] Derive shared secret on chat open
  - [ ] Cache derived keys per conversation

- [ ] **Encrypted Messages**
  - [ ] Encrypt before sending
  - [ ] Decrypt on receive
  - [ ] Handle missing keys gracefully
  - [ ] `EncryptionBadge` component (E2E indicator)

- [ ] **Remaining Chat Features**
  - [ ] Reactions (add/remove/toggle)
  - [ ] Reaction counts display
  - [ ] Reaction breakdown (who reacted)
  - [ ] Reply to message
  - [ ] Edit own message
  - [ ] Delete own message (soft delete)
  - [ ] Typing indicators
  - [ ] Read receipts
  - [ ] Long-press context menu
  - [ ] Image attachments (encrypted)

- [ ] **Verify Crypto Correctness**
  - [ ] Test encryption between two devices
  - [ ] Verify server cannot decrypt
  - [ ] Test key rotation scenario

### Deliverables
- [ ] All messages E2E encrypted
- [ ] Encryption keys in hardware (SE/StrongBox)
- [ ] All chat features working
- [ ] Crypto tests passing
- [ ] Cross-device encryption verified

## Dependencies

**From Phase 2:**
- Electric SQL syncing
- TanStack DB configured
- SeaweedFS for image uploads

**From Phase 1:**
- Secure storage for key references
- Auth flow complete

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| StrongBox not available (old devices) | Fall back to software keystore with warning |
| Encryption breaks message sync | Test incrementally, ciphertext should sync normally |
| Performance issues with decrypt | Decrypt is async, UI should not block |

## Success Criteria

- [ ] Chat fully functional (all 16 features from checklist)
- [ ] Messages E2E encrypted
- [ ] Keys never leave hardware
- [ ] Works offline
- [ ] Syncs reliably
- [ ] 90%+ test coverage on chat
- [ ] 0 lint/type errors
