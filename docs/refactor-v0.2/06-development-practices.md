# Development Practices

## Code Quality: oxlint

Zero tolerance for lint warnings. All rules enforced at CI level.

**Configuration (`.oxlintrc.json`):**

```json
{
  "rules": {
    "correctness": {
      "no-unused-vars": "error",
      "no-undef": "error",
      "no-const-assign": "error",
      "no-dupe-keys": "error",
      "no-self-compare": "error",
      "no-unreachable": "error"
    },
    "perf": {
      "no-delete": "warn",
      "no-accumulating-spread": "warn"
    },
    "suspicious": {
      "no-debugger": "error",
      "no-console": ["error", { "allow": ["warn", "error"] }],
      "no-duplicate-case": "error",
      "no-fallthrough": "error"
    },
    "pedantic": {
      "no-else-return": "warn",
      "prefer-const": "error"
    },
    "typescript": {
      "no-explicit-any": "error",
      "no-non-null-assertion": "error",
      "prefer-ts-expect-error": "error"
    },
    "react": {
      "exhaustive-deps": "error",
      "rules-of-hooks": "error",
      "no-direct-mutation-state": "error"
    }
  },
  "ignorePatterns": ["node_modules", "dist", "*.config.js", "*.config.ts"]
}
```

**Enforcement:**

```bash
# CI pipeline runs this
bun lint  # oxlint with above config

# Pre-commit hook (via husky)
bun lint --fix  # auto-fix what can be fixed
```

**Current issues to fix (from existing codebase):**
- 101 oxlint warnings total
- ~15 `no-explicit-any` violations
- ~20 unused variables
- ~8 console.log statements
- ~12 missing exhaustive-deps

**Goal:** 0 warnings. Block merges on any violation.

## TypeScript Safety

Strict mode everywhere. Zero type assertions.

**tsconfig.json (strict settings):**

```json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "strictFunctionTypes": true,
    "strictBindCallApply": true,
    "strictPropertyInitialization": true,
    "noImplicitThis": true,
    "useUnknownInCatchVariables": true,
    "alwaysStrict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "exactOptionalPropertyTypes": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "noPropertyAccessFromIndexSignature": true
  }
}
```

**Banned patterns (enforced by oxlint + code review):**

| Pattern | Why Banned | Alternative |
|---------|------------|-------------|
| `any` | Defeats type safety | Use `unknown` + type guards |
| `as Type` | Lies to compiler | Use type guards or fix source |
| `obj!` | Non-null assertion | Check for null explicitly |
| `@ts-ignore` | Hides errors | Fix the actual error |
| `@ts-expect-error` | Same | Fix the actual error |
| `@ts-nocheck` | Disables all checks | Never use |
| `// eslint-disable` | Hides lint errors | Fix the actual error |

**Examples of proper type narrowing:**

```typescript
// ❌ BAD: Type assertion
const user = data as User

// ✅ GOOD: Type guard
function isUser(data: unknown): data is User {
  return (
    typeof data === 'object' &&
    data !== null &&
    'id' in data &&
    'email' in data
  )
}
if (isUser(data)) {
  // data is now User
}

// ❌ BAD: Non-null assertion
const name = user.profile!.name

// ✅ GOOD: Explicit check
const name = user.profile?.name ?? 'Unknown'

// ❌ BAD: any
function process(data: any) { ... }

// ✅ GOOD: unknown + validation
function process(data: unknown) {
  const validated = schema.parse(data) // Zod validates
  // validated is now typed
}
```

**Eden type safety:**

```typescript
// Types flow from API automatically via Eden
const { data, error } = await api.profiles[':id'].get({ params: { id } })

// ❌ NEVER do this
const profile = data as Profile  // Defeats the point

// ✅ Eden already provides correct types
if (error) {
  // Handle error (typed)
  return
}
// data is typed as Profile automatically
```

**Enforcement:**

```bash
# CI pipeline
bun typecheck  # tsc --noEmit

# Must pass before merge
# 0 TypeScript errors allowed
```

## Test-Driven Development (TDD)

Refactor with confidence. Write tests first.

**TDD Cycle:**

```
1. Write failing test (RED)
2. Write minimal code to pass (GREEN)
3. Refactor with tests as safety net (REFACTOR)
4. Repeat
```

**What to test:**

| Layer | What to Test | Framework |
|-------|--------------|-----------|
| API routes | Request/response contracts | `bun:test` + `treaty(app)` |
| Services | Business logic | `bun:test` |
| Hooks | State management | `@testing-library/react` |
| Components | User interactions | `@testing-library/react` |
| Encryption | Crypto correctness | `bun:test` |

**Testing strategy for refactor:**

```
Phase 1: Characterization tests (capture current behavior)
├── Write tests for existing API endpoints
├── Write tests for existing service functions
└── Run against current code to establish baseline

Phase 2: TDD for new code
├── Write tests for new useChat() hook FIRST
├── Write tests for new encryption module FIRST
└── Write tests for simplified components FIRST

Phase 3: Migration with safety net
├── Keep old tests passing during migration
├── Old and new tests run in parallel
└── Delete old tests only after new code verified
```

**Test file structure:**

```
apps/
├── api/
│   └── src/
│       └── features/
│           └── chats/
│               ├── index.ts
│               ├── service.ts
│               ├── index.test.ts      # Route tests
│               └── service.test.ts    # Service tests
└── mobile/
    └── src/
        └── hooks/
            ├── useChat.ts
            └── useChat.test.ts        # Hook tests
```

**API test example:**

```typescript
// apps/api/src/features/chats/index.test.ts
import { describe, it, expect, beforeEach } from 'bun:test'
import { treaty } from '@elysiajs/eden'
import { app } from '../../app'

const api = treaty(app)

describe('POST /chats/:id/messages', () => {
  it('returns 401 without auth', async () => {
    const { error } = await api.chats[':id'].messages.post({
      params: { id: 'conv-1' },
      body: { content: 'test' },
    })
    expect(error?.status).toBe(401)
  })

  it('returns 403 if not participant', async () => {
    const { error } = await api.chats[':id'].messages.post({
      params: { id: 'conv-1' },
      body: { content: 'test' },
      headers: { authorization: `Bearer ${nonParticipantToken}` },
    })
    expect(error?.status).toBe(403)
  })

  it('creates message with valid request', async () => {
    const { data, error } = await api.chats[':id'].messages.post({
      params: { id: 'conv-1' },
      body: { content: 'encrypted-content', contentIv: 'nonce' },
      headers: { authorization: `Bearer ${participantToken}` },
    })
    expect(error).toBeNull()
    expect(data?.id).toBeDefined()
    expect(data?.content).toBe('encrypted-content')
  })
})
```

**Hook test example:**

```typescript
// apps/mobile/src/hooks/useChat.test.ts
import { describe, it, expect } from 'bun:test'
import { renderHook, act, waitFor } from '@testing-library/react'
import { useChat } from './useChat'

describe('useChat', () => {
  it('returns empty messages initially', () => {
    const { result } = renderHook(() => useChat('conv-1'))
    expect(result.current.messages).toEqual([])
    expect(result.current.isLoading).toBe(true)
  })

  it('sends message and updates local state immediately', async () => {
    const { result } = renderHook(() => useChat('conv-1'))

    await act(async () => {
      result.current.send({ text: 'Hello' })
    })

    // Local-first: appears immediately
    await waitFor(() => {
      expect(result.current.messages).toHaveLength(1)
      expect(result.current.messages[0].content).toBe('Hello')
    })
  })

  it('handles offline gracefully', async () => {
    // Simulate offline
    const { result } = renderHook(() => useChat('conv-1'))

    await act(async () => {
      result.current.send({ text: 'Offline message' })
    })

    expect(result.current.isOnline).toBe(false)
    expect(result.current.pendingChanges).toBe(1)
    expect(result.current.messages[0].content).toBe('Offline message')
  })

  it('adds reaction to message', async () => {
    const { result } = renderHook(() => useChat('conv-1'))

    await act(async () => {
      result.current.react({ messageId: 'msg-1', emoji: '👍' })
    })

    await waitFor(() => {
      const msg = result.current.messages.find(m => m.id === 'msg-1')
      expect(msg?.reactions).toContainEqual({ emoji: '👍', count: 1 })
    })
  })
})
```

**Service test example:**

```typescript
// apps/api/src/features/chats/service.test.ts
import { describe, it, expect, beforeEach } from 'bun:test'
import { createMessage, getConversation } from './service'
import { db } from '../../db'

describe('ChatService', () => {
  beforeEach(async () => {
    await db.delete(messages)
    await db.delete(conversations)
  })

  describe('createMessage', () => {
    it('throws if sender not participant', async () => {
      await expect(
        createMessage({
          conversationId: 'conv-1',
          senderId: 'not-a-participant',
          content: 'test',
        })
      ).rejects.toThrow('NOT_PARTICIPANT')
    })

    it('creates message and updates lastMessageAt', async () => {
      const message = await createMessage({
        conversationId: 'conv-1',
        senderId: 'participant-1',
        content: 'encrypted',
        contentIv: 'nonce',
      })

      expect(message.id).toBeDefined()

      const conv = await getConversation('conv-1')
      expect(conv.lastMessageAt).toBe(message.createdAt)
    })
  })
})
```

**Coverage requirements:**

| Layer | Minimum Coverage | Target |
|-------|------------------|--------|
| API routes | 90% | 95% |
| Services | 85% | 90% |
| Hooks | 80% | 90% |
| Components | 70% | 80% |
| **Overall** | **80%** | **85%** |

**CI enforcement:**

```yaml
# .github/workflows/test.yml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: oven-sh/setup-bun@v1

    - run: bun install
    - run: bun lint
    - run: bun typecheck
    - run: bun test --coverage

    - name: Check coverage threshold
      run: |
        COVERAGE=$(bun test --coverage | grep 'All files' | awk '{print $4}' | tr -d '%')
        if (( $(echo "$COVERAGE < 80" | bc -l) )); then
          echo "Coverage $COVERAGE% is below 80% threshold"
          exit 1
        fi
```

**Refactor safety checklist:**

Before deleting any old code:
- [ ] New tests cover same scenarios as old code
- [ ] New tests pass
- [ ] Old tests still pass (if kept temporarily)
- [ ] Manual smoke test on device
- [ ] Coverage didn't decrease
