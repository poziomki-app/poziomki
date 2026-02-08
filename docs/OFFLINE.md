# Offline Layer Architecture

This document describes the offline-first architecture for Poziomki mobile app, enabling users to browse cached data, compose messages, and queue actions while offline.

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         App Layer                                │
├─────────────────────────────────────────────────────────────────┤
│  TanStack Query (v5)                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────────┐  │
│  │   Queries    │  │  Mutations   │  │  Optimistic Updates   │  │
│  │  (cached)    │  │  (queued)    │  │  (instant feedback)   │  │
│  └──────┬───────┘  └──────┬───────┘  └───────────┬───────────┘  │
├─────────┼─────────────────┼──────────────────────┼──────────────┤
│         │                 │                      │              │
│         ▼                 ▼                      ▼              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Offline Service                          ││
│  │  • Network status detection (expo-network)                  ││
│  │  • Mutation queue management (SQLite)                       ││
│  │  • Sync coordination (foreground/reconnect)                 ││
│  └─────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────┤
│                      Storage Layer                              │
│  ┌──────────────────┐  ┌──────────────────────────────────────┐ │
│  │  AsyncStorage    │  │  SQLite (expo-sqlite)                │ │
│  │  Query Cache     │  │  • mutation_queue                    │ │
│  │  (TanStack)      │  │  • sync_metadata                     │ │
│  └──────────────────┘  └──────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                      Image Cache                                │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  expo-image (memory + disk cache)                           ││
│  │  • Automatic caching via cachePolicy="memory-disk"          ││
│  │  • Prefetch on query success                                ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Technology Choices

| Component | Technology | Why |
|-----------|------------|-----|
| Query persistence | TanStack Query Persist + AsyncStorage | Zero changes to existing hooks |
| Mutation queue | expo-sqlite | ACID transactions, crash-safe |
| Network detection | expo-network | Already installed, native accuracy |
| Image caching | expo-image | Already configured, handles auth headers |

### Why Not Other Options?

**WatermelonDB/PowerSync**: Too complex for our needs. Requires server-side sync protocol changes. Better suited for true offline-first apps with complex conflict resolution.

**MMKV for queries**: Faster than AsyncStorage but requires additional native dependency. Can migrate later if performance is an issue.

**Redux Persist**: Would require migrating from TanStack Query. Not worth the effort.

## Data Categories

### High Priority (Always Cached)

| Data | Cache Duration | Sync Strategy |
|------|---------------|---------------|
| My profile | 24 hours | Sync on app foreground |
| Events list | 24 hours | Background refresh, pull-to-refresh |
| Conversations list | 24 hours | Sync on reconnect |
| Tags | 24 hours | Rarely changes |

### Medium Priority (Cached on View)

| Data | Cache Duration | Sync Strategy |
|------|---------------|---------------|
| Event details | 24 hours | Fetch on open, use cache if offline |
| Other profiles | 24 hours | Fetch on open |
| Messages (per conversation) | 24 hours | Infinite scroll pages cached |

### Low Priority (Best Effort)

| Data | Cache Duration | Notes |
|------|---------------|-------|
| Recommendations | 24 hours | Can be stale |
| Search results | Not cached | Too dynamic |

## Mutation Queue

### Supported Offline Mutations

| Mutation | Queue Support | Notes |
|----------|--------------|-------|
| Send message | Yes | Encrypted locally, queued |
| Create event | Yes | Full payload queued |
| Update event | Yes | Patch payload queued |
| Update profile | Yes | Patch payload queued |
| Attend event | Yes | Simple status change |
| Add reaction | No | Too transient |
| Delete message | No | Requires immediate confirmation |

### Queue Schema

```sql
CREATE TABLE mutation_queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  client_id TEXT NOT NULL UNIQUE,    -- UUID for deduplication
  mutation_type TEXT NOT NULL,        -- 'send_message', 'create_event', etc.
  payload TEXT NOT NULL,              -- JSON serialized data
  entity_id TEXT,                     -- Related entity for conflict check
  status TEXT DEFAULT 'pending',      -- pending | processing | failed
  retry_count INTEGER DEFAULT 0,
  last_error TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
```

### Processing Flow

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│   Queue     │────▶│   Process    │────▶│   Success   │
│   Mutation  │     │   (online)   │     │   Remove    │
└─────────────┘     └──────────────┘     └─────────────┘
                           │
                           ▼ (error)
                    ┌──────────────┐
                    │   Retry?     │
                    │  (max 5x)    │
                    └──────────────┘
                      │         │
                 yes  ▼         ▼ no
              ┌────────┐   ┌─────────┐
              │ Backoff│   │  Mark   │
              │  Wait  │   │ Failed  │
              └────────┘   └─────────┘
```

## Sync Triggers

| Trigger | Action |
|---------|--------|
| App foreground | Process queue, invalidate stale queries |
| Network restored | Process queue |
| Pull-to-refresh | Process queue, force refetch |
| WebSocket reconnect | Process queue (chat messages) |

Minimum interval between syncs: 30 seconds (prevent excessive API calls).

## Conflict Resolution: Server Wins

When a queued mutation fails with a 4xx error (conflict, not found, forbidden):

1. Remove mutation from queue (discard local change)
2. Invalidate related queries
3. Let UI show server state

**Rationale**: Simplicity. Complex merge UIs are confusing for users. Server state is authoritative.

## Optimistic Updates

For immediate UI feedback while offline:

```typescript
// Example: Send message
const mutation = useMutation({
  mutationFn: async (text) => {
    if (!isOnline) {
      await queueMutation({ type: 'send_message', payload: { text } });
      return { id: clientId, text, _isPending: true };
    }
    return api.messages.post({ text });
  },
  onMutate: async (text) => {
    // Cancel in-flight queries
    await queryClient.cancelQueries(['messages']);
    // Snapshot previous
    const previous = queryClient.getQueryData(['messages']);
    // Optimistically add message
    queryClient.setQueryData(['messages'], old => [...old, { text, _pending: true }]);
    return { previous };
  },
  onError: (err, text, context) => {
    // Rollback on failure
    queryClient.setQueryData(['messages'], context.previous);
  },
});
```

## Image Caching

### Current Implementation

`AuthenticatedImage` component uses expo-image with:
- `cachePolicy="memory-disk"` - Persists images to disk
- Authorization header injection for CDN access

### Enhancement: Prefetch on Query Success

```typescript
// After fetching events list
const { data: events } = useEvents();

useEffect(() => {
  if (events) {
    const imageUrls = events
      .map(e => e.coverImage)
      .filter(Boolean);
    prefetchImages(imageUrls);
  }
}, [events]);
```

### Offline Image Placeholder

When offline and image not in cache:
- Show subtle placeholder icon
- Don't show broken image or error state

## User Experience

### Offline Indicators

1. **Banner**: "You're offline - showing cached data" at screen top
2. **Message status**: Clock icon for pending, retry button for failed
3. **Sync badge**: "3 pending" on relevant tabs

### What Users Can Do Offline

| Action | Supported | Notes |
|--------|-----------|-------|
| Browse events | Yes | Cached list + details |
| View profiles | Yes | If previously viewed |
| Read messages | Yes | Cached conversation history |
| Send messages | Yes | Queued, sent when online |
| Create events | Yes | Queued |
| Edit profile | Yes | Queued |
| Join event | Yes | Queued |
| Search | No | Requires server |
| View recommendations | Partial | Shows cached if available |

### What Happens on Reconnect

1. Offline banner disappears
2. Queued mutations process in background
3. Pending message indicators update to "sent"
4. Stale queries refresh automatically

## Implementation Phases

### Phase 1: Query Persistence + Network Detection
- Add AsyncStorage persister to QueryClient
- Create network status hook with expo-network
- Add offline banner component

### Phase 2: Mutation Queue
- Create SQLite database and schema
- Implement queue service with retry logic
- Modify mutation hooks for offline support

### Phase 3: Chat Offline Support
- Integrate message sending with queue
- Add message status indicators
- Connect WebSocket reconnect to sync

### Phase 4: Image Prefetch + Polish
- Add image prefetch on query success
- Offline placeholder for uncached images
- Pull-to-refresh sync integration

## Dependencies to Add

```json
{
  "@react-native-async-storage/async-storage": "^2.0.0",
  "@tanstack/query-async-storage-persister": "^5.0.0",
  "@tanstack/react-query-persist-client": "^5.0.0",
  "expo-sqlite": "~15.0.0"
}
```

## File Structure

```
apps/mobile/src/
├── lib/
│   ├── offline/
│   │   ├── db.ts              # SQLite init + schema
│   │   ├── mutation-queue.ts  # Queue operations
│   │   ├── sync-service.ts    # Sync coordination
│   │   └── image-prefetch.ts  # Image cache helpers
│   ├── network.ts             # Network state hook
│   └── query-client.ts        # (modified) Add persistence
├── hooks/
│   └── use-network.ts         # useIsOnline convenience hook
└── components/
    └── shared/
        ├── OfflineBanner.tsx     # Offline indicator
        └── PendingSyncBadge.tsx  # Pending count badge
```

## Testing Strategy

### Manual Testing Scenarios

1. **Read offline**: Open app → browse → airplane mode → verify data visible
2. **Queue mutation**: Airplane mode → create event → online → verify synced
3. **Chat offline**: Airplane mode → send message → verify pending → online → verify sent
4. **Conflict**: Edit event offline → someone else deletes it → online → verify graceful handling
5. **Image cache**: View event → airplane mode → verify cover image shows

### Automated Tests

- Unit tests for mutation queue operations
- Integration tests for sync service
- Mock network state for offline scenarios
