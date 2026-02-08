# Architecture

## Stack

```
Backend:    Elysia + Eden (unchanged)
Database:   Postgres 18 + Drizzle (Chainguard hardened)
Cache:      Dragonfly (Chainguard hardened)
Storage:    SeaweedFS (S3-compatible, hardened config)
Sync:       Electric SQL (self-hosted, Postgres logical replication)
Frontend:   LynxJS + ReactLynx
Client DB:  TanStack DB (in-memory reactive store, few KBs)
Styling:    Native CSS + CSS variables (dark mode only)
Routing:    TanStack Router (file-based, type-safe)
Data:       TanStack Query (profiles, events) + TanStack DB (chats)
Images:     Landscapist Core (fastest, cross-platform)
Encryption: Native platform crypto (Secure Enclave / StrongBox)
OTA:        Self-hosted bundle updates
```

## Screens (12 total)

```
app/
├── (auth)/
│   ├── login.tsx           # Login + OTP in one flow
│   └── onboarding.tsx      # Combined basic+profile+interests (single screen)
│
├── (tabs)/
│   ├── _layout.tsx
│   ├── discover.tsx        # Profiles feed
│   ├── events.tsx          # Events list
│   ├── chats.tsx           # Conversations list
│   └── me.tsx              # Settings/profile
│
├── profile/
│   ├── [id].tsx            # Profile detail (modal)
│   └── edit.tsx            # Edit own profile
│
├── event/
│   ├── [id].tsx            # Event detail (modal)
│   └── create.tsx          # Create/edit event
│
├── chat/
│   └── [id].tsx            # Chat screen
│
└── privacy.tsx             # Privacy policy (legal requirement)
```

### Screens Cut

| Removed | Replacement |
|---------|-------------|
| `register.tsx` | Merged into `login.tsx` |
| `verify.tsx` | Inline OTP in login flow |
| `(onboarding)/basic.tsx` | Single `onboarding.tsx` |
| `(onboarding)/profile.tsx` | Single `onboarding.tsx` |
| `(onboarding)/interests.tsx` | Single `onboarding.tsx` |
| `chat/new.tsx` | Tap profile → opens chat directly |
| `profile/index.tsx` | Use `profile/[id].tsx` |

## Hooks (15 total)

### API Layer (5)

```typescript
// apps/mobile/src/hooks/api/

useProfiles()      // Discovery feed + profile detail + bookmarks
useEvents()        // List + detail + create/edit + attendance
useChats()         // Conversations list + messages (UNIFIED)
useAccount()       // Auth + settings + account management
useUpload()        // File uploads with progress
```

### Feature Hooks (6)

```typescript
// apps/mobile/src/hooks/

useChat(conversationId)   // Messages, send, reactions, WebSocket
useEvent(eventId)         // Detail, attend, leave, chat access
useProfile(profileId)     // Detail, bookmark, start chat
useDiscovery()            // Infinite scroll + filters
useOnboarding()           // Multi-step form state machine
useAuth()                 // Login/logout/session/OTP
```

### Utility Hooks (4)

```typescript
// apps/mobile/src/hooks/

useForm<T>(schema)        // Generic Zod-validated form
useInfiniteList<T>()      // Pagination/infinite scroll pattern
useChatDb()               // TanStack DB instance for chat (local-first)
useSecureStorage()        // Native secure storage wrapper
```

### Hooks Eliminated (27)

**Chat hooks merged into `useChat()`:**
- ~~useChatActions~~
- ~~useChatEffects~~
- ~~useChatInput~~
- ~~useChatNavigation~~
- ~~useChatScreenData~~
- ~~useEventLiveChatActions~~
- ~~useMessageContextActions~~
- ~~useMessageListItem~~
- ~~useMessageListScroll~~
- ~~useReactionModal~~
- ~~useWebSocketConnection~~
- ~~useWebSocketEventHandlers~~
- ~~message-handlers.ts~~

**Auth hooks merged into `useAuth()`:**
- ~~useRegisterValidation~~
- ~~useEmailTakenRecovery~~
- ~~useLoginForm~~
- ~~useRegisterForm~~
- ~~useOtpInput~~

**Event hooks merged:**
- ~~useEventsFilter~~ → `useDiscovery()`
- ~~useEventDetailActions~~ → `useEvent()`
- ~~useEventDateFields~~ → `useForm()`
- ~~useEventEditForm~~ → `useForm()`
- ~~useCreateEventForm~~ → `useForm()`

**Other consolidated:**
- ~~use-profile-creation~~ → `useOnboarding()`
- ~~use-tag-hierarchy~~ → inline
- ~~useNavbarScroll~~ → CSS
- ~~use-chat-socket~~ → Electric SQL sync (no WebSocket)
- ~~use-bookmarks~~ → `useProfiles()`
- ~~use-tag-selection~~ → `useForm()`
- ~~use-profile-image-picker~~ → `useUpload()`
- ~~useKeyboard~~ → native handling

**WebSocket eliminated entirely:**
- Chat sync handled by Electric SQL
- No WebSocket connection management
- No manual reconnection logic
- No message deduplication needed

## Components (35 total)

### Design System (10)

```typescript
// apps/mobile/src/components/ui/

Button        // Primary, secondary, ghost variants
Input         // Text, password, multiline
Text          // Typography with semantic variants
Card          // Container with shadow/border
Avatar        // Image with fallback initials
Modal         // Bottom sheet / dialog
List          // Virtualized list wrapper
Badge         // Status indicators
Skeleton      // Loading placeholder (single generic)
Icon          // Icon wrapper with consistent sizing
```

### Feature Components (25)

```typescript
// apps/mobile/src/components/

// Auth (3)
auth/
├── LoginForm.tsx        // Email + OTP combined
├── OtpInput.tsx         // 6-digit code input
└── OnboardingForm.tsx   // Multi-step with progress

// Profile (4)
profile/
├── ProfileCard.tsx      // Card in discovery feed
├── ProfileDetail.tsx    // Full profile view
├── ProfileEdit.tsx      // Edit form
└── TagSelector.tsx      // Interest tag picker

// Event (5)
event/
├── EventCard.tsx        // Card in events list
├── EventDetail.tsx      // Full event view
├── EventForm.tsx        // Create/edit form
├── AttendeeList.tsx     // List of attendees
└── DateTimePicker.tsx   // Date/time selection

// Chat (8)
chat/
├── ChatList.tsx         // Conversations list
├── ChatItem.tsx         // Single conversation row
├── ChatScreen.tsx       // Main chat view container
├── MessageList.tsx      // Virtualized message list
├── MessageBubble.tsx    // Single message (simplified)
├── ChatInput.tsx        // Message composer
├── ReactionPicker.tsx   // Emoji picker (inline, not modal)
└── EncryptionBadge.tsx  // E2E indicator

// Shared (5)
shared/
├── PhotoPicker.tsx      // Camera/gallery picker
├── ImageGallery.tsx     // Image viewer
├── EmptyState.tsx       // Empty list placeholder
├── ErrorBoundary.tsx    // Error handling
└── TabBar.tsx           // Bottom navigation
```

### Components Cut (49)

From chat/ (12 cut):
- ~~ReactionBreakdownModal~~ (327 LOC) → inline counts
- ~~MessageContextMenu~~ (270 LOC) → long-press actions
- ~~ConversationItem~~ → renamed to ChatItem
- ~~ChatScreenContent~~ → merged into ChatScreen
- ~~MessageActions~~ → inline in MessageBubble
- ~~TypingIndicator~~ → simplified
- ~~ReadReceipts~~ → simplified
- ~~ReplyPreview~~ → simplified
- ~~MessageEditor~~ → simplified
- ~~ForwardModal~~ → cut feature
- ~~LinkPreview~~ → cut feature
- ~~ChatHeader~~ → inline

From shared/ (7 cut):
- ~~ScrollIndicator~~ (178 LOC) → native scroll
- ~~PhotoPickerModal~~ (294 LOC) → simplified PhotoPicker
- Multiple modal variants → single Modal component

From form/ (7 cut):
- ~~DegreeAutocomplete~~ (181 LOC) → simplified
- Individual form inputs → use design system

From skeleton/ (3 cut):
- Multiple skeleton variants → single Skeleton component

## API Simplification

### Current Structure (8 features, sprawling)

```
features/
├── auth/           (7 files, 1,220 LOC)
├── chats/          (15+ files across service/, websocket/)
├── profiles/       (3 files, 503 LOC)
├── events/         (12 files across service/)
├── tags/           (3 files)
├── degrees/        (3 files)
├── matching/       (2 files)
└── uploads/        (2 files, 381 LOC)
```

### New Structure (5 features, flat)

```
features/
├── auth/
│   ├── index.ts      # Routes (~150 LOC)
│   ├── service.ts    # Logic (~200 LOC, merged account.ts)
│   └── schema.ts     # Zod schemas
│
├── profiles/
│   ├── index.ts      # Routes (~80 LOC)
│   ├── service.ts    # Logic (~200 LOC, includes tags/degrees/matching)
│   └── schema.ts
│
├── events/
│   ├── index.ts      # Routes (~100 LOC)
│   ├── service.ts    # Logic (~200 LOC, merged 9 files)
│   └── schema.ts
│
├── chats/
│   ├── index.ts      # Routes (~150 LOC)
│   ├── service.ts    # Logic (~250 LOC, merged loaders/transformers)
│   ├── websocket.ts  # WebSocket handler (~150 LOC, single file)
│   └── schema.ts
│
└── uploads/
    ├── index.ts      # Routes (~80 LOC)
    └── service.ts    # Logic (~100 LOC)
```

### Features Eliminated

| Removed | Merged Into |
|---------|-------------|
| `tags/` | `profiles/service.ts` |
| `degrees/` | `profiles/service.ts` |
| `matching/` | `profiles/service.ts` |
| `chats/service/loaders/` | `chats/service.ts` |
| `chats/service/transformers/` | `chats/service.ts` |
| `chats/service/conversations/` | `chats/service.ts` |
| `chats/websocket/` (folder) | `chats/websocket.ts` (single file) |
| `events/service/` (9 files) | `events/service.ts` |
| `auth/account.ts` | `auth/service.ts` |
| `auth/account-mappers.ts` | `auth/service.ts` |
