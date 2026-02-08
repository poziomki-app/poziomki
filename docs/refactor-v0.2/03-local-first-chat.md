# Local-First Chat Architecture

## Why Local-First

| Benefit | Impact |
|---------|--------|
| **Offline support** | Chat works without network |
| **Instant UI** | Sub-millisecond reads from local store |
| **Reduced server load** | Sync deltas, not full data |
| **Better UX** | No loading spinners for cached data |
| **Resilience** | Network failures don't break the app |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Mobile App                           │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐ │
│  │ TanStack    │    │ TanStack DB │    │ Encryption      │ │
│  │ Query       │    │ (Chats)     │    │ Layer           │ │
│  │ (Profiles,  │    │             │    │ (X25519+AES)    │ │
│  │  Events)    │    │ Local-first │    │                 │ │
│  └──────┬──────┘    └──────┬──────┘    └────────┬────────┘ │
│         │                  │                     │          │
│         ▼                  ▼                     ▼          │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    Eden Client                          ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Elysia API                              │
└─────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────┐
│   PostgreSQL    │ │  Electric SQL   │ │     SeaweedFS       │
│   (Primary)     │ │  (Sync Server)  │ │   (Object Storage)  │
└─────────────────┘ └─────────────────┘ └─────────────────────┘
```

## Electric SQL Setup

**Server (docker-compose.yml):**

```yaml
services:
  electric:
    image: electricsql/electric:latest
    environment:
      DATABASE_URL: postgres://poziomki:${DB_PASSWORD}@postgres:5432/poziomki
      ELECTRIC_WRITE_TO_PG_MODE: direct_writes
      AUTH_MODE: secure
      AUTH_JWT_KEY: ${ELECTRIC_JWT_KEY}
    ports:
      - "127.0.0.1:5133:5133"
    depends_on:
      postgres:
        condition: service_healthy
```

**Postgres (one-time setup):**

```sql
-- Enable logical replication
ALTER SYSTEM SET wal_level = 'logical';
-- Restart Postgres, then:
ALTER USER poziomki WITH REPLICATION;

-- Enable Electric on messages table
ALTER TABLE messages REPLICA IDENTITY FULL;
ALTER TABLE conversations REPLICA IDENTITY FULL;
ALTER TABLE conversation_participants REPLICA IDENTITY FULL;
ALTER TABLE message_reactions REPLICA IDENTITY FULL;
```

## TanStack DB Integration

```typescript
// apps/mobile/src/lib/chat-db.ts

import { createCollection, createDatabase } from '@anthropic/tanstack-db'
import { electricSync } from '@anthropic/tanstack-db-electric'

// Define synced collections
export const messagesCollection = createCollection({
  id: 'messages',
  schema: {
    id: 'string',
    conversationId: 'string',
    senderId: 'string',
    content: 'string',        // Encrypted ciphertext
    contentIv: 'string',      // Encryption nonce
    createdAt: 'string',
    updatedAt: 'string',
    deletedAt: 'string?',
  },
  primaryKey: 'id',
  indexes: ['conversationId', 'createdAt'],
})

export const conversationsCollection = createCollection({
  id: 'conversations',
  schema: {
    id: 'string',
    type: 'string',           // 'personal' | 'event'
    eventId: 'string?',
    lastMessageAt: 'string?',
    createdAt: 'string',
  },
  primaryKey: 'id',
})

export const reactionsCollection = createCollection({
  id: 'reactions',
  schema: {
    id: 'string',
    messageId: 'string',
    profileId: 'string',
    emoji: 'string',
    createdAt: 'string',
  },
  primaryKey: 'id',
  indexes: ['messageId'],
})

// Create database with Electric sync
export const chatDb = createDatabase({
  collections: [messagesCollection, conversationsCollection, reactionsCollection],
  sync: electricSync({
    url: process.env.ELECTRIC_URL ?? 'https://electric.poziomki.app',
    // Auth token from session
    getToken: () => getAuthToken(),
  }),
})
```

## useChat Hook (Local-First, Full Features)

Single hook replaces 14 current hooks while keeping ALL features:

```typescript
// apps/mobile/src/hooks/useChat.ts

import { useLiveQuery, useMutation } from '@tanstack/db-react'
import { chatDb, messagesCollection, reactionsCollection, typingCollection } from '../lib/chat-db'
import { useChatEncryption } from './useChatEncryption'

export function useChat(conversationId: string) {
  const crypto = useChatEncryption(conversationId)
  const myProfileId = getProfileId()

  // ═══════════════════════════════════════════════════════════
  // MESSAGES - Live query, works offline
  // ═══════════════════════════════════════════════════════════
  const messagesQuery = useLiveQuery(() =>
    chatDb.query(messagesCollection)
      .where('conversationId', '=', conversationId)
      .orderBy('createdAt', 'desc')
      .limit(100)
      .toArray()
  )

  // Decrypt messages for display
  const messages = useMemo(() =>
    messagesQuery.data
      ?.filter(m => !m.deletedAt)
      .map(m => ({
        ...m,
        content: crypto.decrypt(m.content, m.contentIv),
        reactions: getReactionsForMessage(m.id),
        replyTo: m.replyToId ? messagesQuery.data.find(r => r.id === m.replyToId) : null,
      })) ?? [],
    [messagesQuery.data, crypto]
  )

  // ═══════════════════════════════════════════════════════════
  // REACTIONS - Live query for reaction counts and breakdown
  // ═══════════════════════════════════════════════════════════
  const reactionsQuery = useLiveQuery(() =>
    chatDb.query(reactionsCollection)
      .where('conversationId', '=', conversationId)
      .toArray()
  )

  const getReactionsForMessage = (messageId: string) => {
    const messageReactions = reactionsQuery.data?.filter(r => r.messageId === messageId) ?? []
    // Group by emoji with profiles
    return Object.entries(
      messageReactions.reduce((acc, r) => {
        acc[r.emoji] = acc[r.emoji] || []
        acc[r.emoji].push(r.profileId)
        return acc
      }, {} as Record<string, string[]>)
    ).map(([emoji, profileIds]) => ({ emoji, count: profileIds.length, profileIds }))
  }

  // ═══════════════════════════════════════════════════════════
  // TYPING INDICATORS - Live query
  // ═══════════════════════════════════════════════════════════
  const typingQuery = useLiveQuery(() =>
    chatDb.query(typingCollection)
      .where('conversationId', '=', conversationId)
      .where('profileId', '!=', myProfileId)
      .where('expiresAt', '>', new Date().toISOString())
      .toArray()
  )

  const typingProfiles = typingQuery.data?.map(t => t.profileId) ?? []

  // ═══════════════════════════════════════════════════════════
  // READ RECEIPTS - Track last read per participant
  // ═══════════════════════════════════════════════════════════
  const readReceiptsQuery = useLiveQuery(() =>
    chatDb.query(readReceiptsCollection)
      .where('conversationId', '=', conversationId)
      .toArray()
  )

  // ═══════════════════════════════════════════════════════════
  // MUTATIONS - All write operations
  // ═══════════════════════════════════════════════════════════

  // Send message
  const send = useMutation({
    mutationFn: async ({ text, replyToId }: { text: string; replyToId?: string }) => {
      const encrypted = await crypto.encrypt(text)
      const message = {
        id: crypto.randomUUID(),
        conversationId,
        senderId: myProfileId,
        content: encrypted.ciphertext,
        contentIv: encrypted.nonce,
        replyToId: replyToId ?? null,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      }
      await chatDb.insert(messagesCollection, message)
      return message
    }
  })

  // Edit message
  const editMessage = useMutation({
    mutationFn: async ({ messageId, text }: { messageId: string; text: string }) => {
      const encrypted = await crypto.encrypt(text)
      await chatDb.update(messagesCollection, messageId, {
        content: encrypted.ciphertext,
        contentIv: encrypted.nonce,
        updatedAt: new Date().toISOString(),
        editedAt: new Date().toISOString(),
      })
    }
  })

  // Delete message (soft delete)
  const deleteMessage = useMutation({
    mutationFn: async (messageId: string) => {
      await chatDb.update(messagesCollection, messageId, {
        deletedAt: new Date().toISOString(),
      })
    }
  })

  // React to message
  const react = useMutation({
    mutationFn: async ({ messageId, emoji }: { messageId: string; emoji: string }) => {
      // Toggle: remove if exists, add if not
      const existing = reactionsQuery.data?.find(
        r => r.messageId === messageId && r.profileId === myProfileId && r.emoji === emoji
      )
      if (existing) {
        await chatDb.delete(reactionsCollection, existing.id)
      } else {
        await chatDb.insert(reactionsCollection, {
          id: crypto.randomUUID(),
          conversationId,
          messageId,
          profileId: myProfileId,
          emoji,
          createdAt: new Date().toISOString(),
        })
      }
    }
  })

  // Set typing indicator
  const setTyping = useMutation({
    mutationFn: async (isTyping: boolean) => {
      if (isTyping) {
        await chatDb.upsert(typingCollection, {
          id: `${conversationId}:${myProfileId}`,
          conversationId,
          profileId: myProfileId,
          expiresAt: new Date(Date.now() + 5000).toISOString(), // 5 second TTL
        })
      } else {
        await chatDb.delete(typingCollection, `${conversationId}:${myProfileId}`)
      }
    }
  })

  // Mark as read
  const markAsRead = useMutation({
    mutationFn: async (messageId: string) => {
      await chatDb.upsert(readReceiptsCollection, {
        id: `${conversationId}:${myProfileId}`,
        conversationId,
        profileId: myProfileId,
        lastReadMessageId: messageId,
        readAt: new Date().toISOString(),
      })
    }
  })

  // ═══════════════════════════════════════════════════════════
  // SYNC STATUS
  // ═══════════════════════════════════════════════════════════
  const syncStatus = chatDb.getSyncStatus()

  return {
    // Data
    messages,
    typingProfiles,
    readReceipts: readReceiptsQuery.data ?? [],
    isLoading: messagesQuery.isLoading,

    // Actions
    send: send.mutate,
    editMessage: editMessage.mutate,
    deleteMessage: deleteMessage.mutate,
    react: react.mutate,
    setTyping: setTyping.mutate,
    markAsRead: markAsRead.mutate,

    // Status
    isSending: send.isPending,
    isOnline: syncStatus.isConnected,
    pendingChanges: syncStatus.pendingCount,
  }
}
```

**Result:** 14 hooks → 1 hook. All features preserved.

## Sync Shapes (What Data Syncs)

```typescript
// apps/mobile/src/lib/chat-sync.ts

import { chatDb } from './chat-db'

// Define what data syncs to this device
export async function setupChatSync(profileId: string) {
  // Only sync conversations this user participates in
  await chatDb.syncShape({
    table: 'conversations',
    where: `id IN (
      SELECT conversation_id FROM conversation_participants
      WHERE profile_id = '${profileId}'
    )`,
  })

  // Only sync messages from those conversations
  await chatDb.syncShape({
    table: 'messages',
    where: `conversation_id IN (
      SELECT conversation_id FROM conversation_participants
      WHERE profile_id = '${profileId}'
    )`,
  })

  // Only sync reactions on those messages
  await chatDb.syncShape({
    table: 'message_reactions',
    where: `message_id IN (
      SELECT id FROM messages WHERE conversation_id IN (
        SELECT conversation_id FROM conversation_participants
        WHERE profile_id = '${profileId}'
      )
    )`,
  })
}
```

## Conflict Resolution

TanStack DB + Electric SQL use CRDTs for automatic conflict resolution:

| Conflict Type | Resolution |
|---------------|------------|
| Concurrent edits | Last-write-wins by timestamp |
| Concurrent deletes | Delete wins |
| Reaction toggle | Converges to consistent state |

No manual conflict handling needed.

## Offline Indicators

```tsx
// apps/mobile/src/components/chat/SyncStatus.tsx

export function SyncStatus() {
  const { isOnline, pendingChanges } = useChatSyncStatus()

  if (isOnline && pendingChanges === 0) return null

  return (
    <view class="sync-status">
      {!isOnline && (
        <view class="sync-offline">
          <Icon name="cloud-off" />
          <text>Offline</text>
        </view>
      )}
      {pendingChanges > 0 && (
        <view class="sync-pending">
          <Icon name="cloud-upload" />
          <text>{pendingChanges} pending</text>
        </view>
      )}
    </view>
  )
}
```

```css
.sync-status {
  flex-direction: row;
  align-items: center;
  gap: var(--spacing-sm);
  padding: var(--spacing-xs) var(--spacing-sm);
  background: var(--color-surface-elevated);
  border-radius: var(--radius-full);
}

.sync-offline {
  color: var(--color-text-muted);
}

.sync-pending {
  color: var(--color-accent);
}
```

## Chat Simplification Summary

### Current Complexity

```
14 hooks + 20 components + WebSocket handlers + message transformers
+ encryption layer + reaction system + typing indicators + read receipts
+ reply threading + message editing + context menus
```

### New Architecture (Local-First)

**Key change:** Instead of WebSocket + TanStack Query, we use **TanStack DB + Electric SQL**.

| Old Approach | New Approach |
|--------------|--------------|
| WebSocket for real-time | Electric SQL sync |
| TanStack Query for fetching | TanStack DB live queries |
| Server round-trip per action | Local-first, sync in background |
| Optimistic updates (complex) | True local state (simple) |

### All Chat Features (Kept)

Same features, simpler implementation:

| Feature | Current (Complex) | New (Simple) |
|---------|-------------------|--------------|
| Typing indicators | WebSocket + custom hooks | Electric SQL sync + single hook |
| Read receipts | Multiple hooks | Single query in `useChat()` |
| Message editing | Separate hook + component | Inline in `useChat()` |
| Reactions | 3 hooks + modal | Single hook + inline picker |
| Reaction breakdown | 327 LOC modal | Simple popover |
| Reply threading | Multiple components | Single `ReplyPreview` |
| Mentions | Custom parser | Simple regex + lookup |
| Message context menu | 270 LOC component | Native long-press menu |

**Principle:** Keep every feature, reduce code by 50%+.
