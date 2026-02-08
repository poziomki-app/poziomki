# Code Review Findings - Poziomki Codebase

This document catalogs code quality issues, suspicious patterns, and high-impact areas needing refactoring.

---

## Critical Security Issues

### 1. Fake Encryption - Base64 Only (HIGH IMPACT)
**File:** `apps/mobile/src/lib/crypto.ts:59-78`

The "encryption" is a **placeholder that only does base64 encoding** - no actual cryptography.

```typescript
// Line 59-64
export function encryptMessage(plaintext: string): { content: string; contentIv: string } {
	const iv = getRandomBytes(CRYPTO_CONSTANTS.IV_LENGTH);
	const content = uint8ToBase64(stringToUint8(plaintext)); // Just base64!
	const contentIv = uint8ToBase64(iv);
	return { content, contentIv };
}
```

**Issue:** Messages are stored/transmitted as base64, not encrypted. The IV is generated but **never used**.

### 2. Math.random() for Cryptographic Purposes
**File:** `apps/mobile/src/lib/crypto.ts:18-24`

Falls back to `Math.random()` for generating "random" bytes for encryption.

```typescript
// Line 18-24
} else {
	// Fallback for older environments
	for (let i = 0; i < length; i++) {
		bytes[i] = Math.floor(Math.random() * 256);
	}
}
```

**Issue:** `Math.random()` is NOT cryptographically secure. This should fail hard instead of using weak randomness.

### 3. Token in WebSocket URL Query String
**File:** `apps/mobile/src/hooks/chat/useWebSocketConnection.ts:23-26`

```typescript
function buildWsUrl(token: string): string {
	const baseUrl = getApiBaseUrl().replace(/^http/, 'ws');
	return `${baseUrl}/ws/chat?token=${encodeURIComponent(token)}`;
}
```

**Issue:** Auth tokens in URLs are logged in server access logs, browser history, and potentially cached. Should use WebSocket authentication handshake or cookies instead.

---

## Placeholder Code / TODOs in Production

### 4. Matching Algorithm is Just "Newest Profiles"
**File:** `apps/api/src/features/matching/service.ts:14-17`

```typescript
// TODO: [MATCHING] Current algorithm is a placeholder - returns newest profiles only.
// Needs: tag-based matching, compatibility scoring, preference filtering (age, interests),
// interaction history tracking, and recommendation caching.
```

**Impact:** Recommendations are meaningless - just returns newest profiles.

### 5. Bookmarks Not Synced to Server
**Files:**
- `apps/mobile/src/hooks/use-bookmarks.ts:4`
- `apps/mobile/src/lib/bookmark-store.ts:4`

```typescript
// TODO: [BOOKMARKS] Client-side only - bookmarks stored in AsyncStorage.
// TODO: [BOOKMARKS] Local storage only - not synced to server.
```

**Impact:** Users lose bookmarks when switching devices.

### 6. Search is Client-Side Only
**File:** `apps/mobile/src/app/(tabs)/index.tsx:86`

```typescript
// TODO: [SEARCH] Profile search is client-side only, limited to 10 fetched results.
```

**Impact:** Search only works on first 10 profiles fetched.

---

## Architecture & Pattern Issues

### 7. Object.assign Mutating Function Arguments
**File:** `apps/api/src/features/matching/service.ts:91-93`

```typescript
return profileResults.map((profile) =>
	Object.assign(profile, { tags: tagsByProfile.get(profile.id) || [] })
);
```

**Issue:** Mutates the original `profile` object from the database query. Should create a new object.

### 8. Object.assign on WebSocket Context
**File:** `apps/api/src/features/chats/websocket/index.ts:49-52`

```typescript
Object.assign(context, {
	userId: sessionPayload.user.id,
	profileId: profile.id,
});
```

**Issue:** Mutating Elysia's context object directly is fragile. Should use proper store/state mechanism.

### 9. Type Casting in Profile Tags
**File:** `apps/api/src/features/profiles/service.ts:80`

```typescript
return tagsResult.map((t) => Object.assign(t, { scope: t.scope as ProfileTag['scope'] }));
```

**Issue:** Double issue - mutates result and uses type assertion.

### 10. Database Plugin as Global Singleton
**File:** `apps/api/src/plugins/database.ts:4`

```typescript
export const database = new Elysia({ name: 'database' }).decorate('db', db);
```

**ast-grep warning:** `Avoid decorating with non-request scoped values`

**Issue:** `db` is shared across all requests. While Drizzle handles this, it's not the Elysia pattern.

---

## Complex Functions Needing Refactoring

### 11. ChatInput Component (113 lines)
**File:** `apps/mobile/src/components/chat/ChatInput.tsx:29`

### 12. EventLiveChat Component (113 lines)
**File:** `apps/mobile/src/components/event/detail/EventLiveChat.tsx:24`

### 13. CreateEventScreen (141 lines)
**File:** `apps/mobile/src/app/event/create.tsx:15`

### 14. useRegisterForm Hook (103 lines)
**File:** `apps/mobile/src/hooks/auth/useRegisterForm.ts:15`

### 15. PhotoPickerModal (135 lines in render)
**File:** `apps/mobile/src/components/shared/PhotoPickerModal.tsx:141`

### 16. EventDetailHeader (97 lines)
**File:** `apps/mobile/src/components/event/detail/EventDetailHeader.tsx:21`

### 17. DateRangePicker (87 lines in render)
**File:** `apps/mobile/src/components/event/form/DateRangePicker.tsx:102`

### 18. useEventDetailActions (87 lines)
**File:** `apps/mobile/src/hooks/event/useEventDetailActions.ts:24`

---

## React Performance Issues

### 19. useCallback with Empty Deps
**File:** `apps/mobile/src/components/ScrollIndicator.tsx:83`

**ast-grep warning:** `useCallback with empty deps - function never updates`

```typescript
const clearScrollTimeout = useCallback(() => {
	if (scrollTimeout.current) {
		clearTimeout(scrollTimeout.current);
		scrollTimeout.current = null;
	}
}, []);
```

**Issue:** This pattern can lead to stale closures in certain scenarios.

### 20. Functions Created in JSX Props
**File:** `apps/mobile/src/components/chat/ChatMessageListItem.tsx:42`

```typescript
// Warning: JSX attribute values should not contain functions created in the same scope
```

---

## Error Handling Issues

### 21. Silent Error Swallowing
**File:** `apps/mobile/src/hooks/api/use-upload.ts:28`

```typescript
const errorData = (await uploadResponse.json().catch(() => ({}))) as Record<string, unknown>;
```

**Issue:** Silently swallows JSON parse errors, hiding potential issues.

### 22. JSON.parse Without Try-Catch
**File:** `apps/mobile/src/hooks/api/use-account.ts:87, 150`

```typescript
const errorData = JSON.parse(responseText);
// ...
const result: { data?: UserDataExport } = JSON.parse(responseText);
```

**Issue:** Could throw on malformed JSON responses.

### 23. Catch Blocks Swallowing Errors
**File:** `apps/api/src/features/auth/account.ts:89-95, 98-104`

```typescript
async function verifyWithBun(password: string, hash: string): Promise<boolean> {
	try {
		return await Bun.password.verify(password, hash);
	} catch {
		return false; // Swallows the error
	}
}
```

**Issue:** Security-sensitive operation swallows errors instead of logging them.

---

## Inconsistent Patterns

### 24. Mixed Error Message Languages
Various files mix Polish and English in error messages:

```typescript
// Polish
throw new Error('Musisz byc zalogowany');
throw new Error('Haslo jest wymagane');

// English
throw new Error('Not authenticated');
throw new Error('No message data returned');
```

### 25. Duplicate Custom Fetch Implementation
**File:** `apps/mobile/src/hooks/api/use-account.ts:71-83`

```typescript
async function authenticatedFetch(path: string, options: RequestInit = {}): Promise<Response> {
```

**Issue:** Doesn't use the shared `api` (Eden) client. Duplicates auth header logic.

---

## Dead/Incomplete Code

### 26. Empty Event Handlers
**File:** `apps/mobile/src/app/chat/[id].tsx:193-202`

```typescript
onSearch={() => {
	/* TODO: [CHAT] Message search not implemented. */
}}
onMenu={() => {
	/* TODO: [CHAT] Chat menu not implemented. */
}}
```

### 27. Incomplete Event Detail Actions
**File:** `apps/mobile/src/app/event/[id].tsx:107-109, 135-141, 148-150`

```typescript
onBookmark={() => {
	/* TODO: bookmark */
}}
onReport={() => {
	/* TODO: [MODERATION] Report event not implemented. */
}}
onLeave: () => {
	/* TODO: [EVENTS] Leave event not implemented. */
}
```

### 28. Missing Chat Hooks
**File:** `apps/mobile/src/hooks/api/use-chats.ts:342-347`

```typescript
// TODO: [CHATS] Missing hooks for chat management features.
// API endpoints exist but hooks not implemented:
// - useLeaveConversation() - DELETE /chats/:id/leave
// - useAddParticipants() - POST /chats/:id/participants
// - useRemoveParticipant() - DELETE /chats/:id/participants/:profileId
```

---

## Logic Oddities

### 29. Redundant setReplyTo(null) Call
**File:** `apps/mobile/src/app/chat/[id].tsx:111-116`

```typescript
const handleSend = useCallback(() => {
	handleSendBase();
	if (!editingMessage) {
		setReplyTo(null); // Also happens in sendMessageFallback line 98
	}
}, [handleSendBase, editingMessage]);
```

**Issue:** `setReplyTo(null)` is called in both places when not editing.

### 30. Lazy Chat Creation in getEventById
**File:** `apps/api/src/features/events/service/index.ts:26-31`

```typescript
// Lazy chat creation for old events without a conversation
if (!event.conversationId && currentProfileId) {
	await getOrCreateEventChat(db, currentProfileId, eventId);
	// Re-fetch to get the new conversationId
	event = await getEventWithRelationsOrThrow(db, eventId);
}
```

**Issue:** GET endpoint has side effects (creates chat). Should be explicit action.

### 31. Hardcoded User Name in Registration
**File:** `apps/mobile/src/hooks/auth/useRegisterForm.ts:103`

```typescript
const result = await signUp.email({ email: fullEmail, password, name: 'User' });
```

**Issue:** Always sends 'User' as name. Name should be optional or collected in onboarding.

### 32. Calendar Button Opens Invite Modal
**File:** `apps/mobile/src/hooks/event/useEventDetailActions.ts:106-108`

```typescript
const handleAddToCalendar = useCallback(() => {
	setInviteOpen(true); // Opens invite modal, not calendar!
}, [setInviteOpen]);
```

**Issue:** Function named `handleAddToCalendar` opens the invite modal instead.

### 33. Type Assertion in Email Recovery
**File:** `apps/mobile/src/hooks/auth/useEmailTakenRecovery.ts:24-29`

```typescript
const data = result.data as {
	token?: string;
	session?: { token?: string };
	data?: { token?: string };
} | null;
const token = data?.token ?? data?.session?.token ?? data?.data?.token;
```

**Issue:** Uses type assertion (`as`) instead of proper type narrowing. The nested optionals also make this fragile.

### 34. Uploads Not Checking Content-Type Header vs Actual Content
**File:** `apps/api/src/features/uploads/index.ts:62-69`

```typescript
// Validate file type
if (!ALLOWED_TYPES.includes(file.type)) {
	throw new HttpError(...)
}
```

**Issue:** Only validates the MIME type from the file object, which comes from the client. Should validate actual file magic bytes to prevent malicious uploads disguised with wrong extensions.

### 35. Public Upload Access Without Auth
**File:** `apps/api/src/features/uploads/index.ts:20-21`

```typescript
// Serve uploaded files (public - UUIDs provide obscurity)
// Auth not required because <img> tags can't send headers
```

**Issue:** Relying on UUID obscurity for security. Anyone who guesses/obtains a filename can access the file. Should consider signed URLs or token-based access.

### 36. Lint Disable Comments
**Files:**
- `apps/mobile/src/styles/unistyles.ts:188`
- `apps/api/src/features/chats/websocket/index.ts:17`
- `apps/mobile/src/components/ProgressBar.tsx:31`
- `apps/mobile/src/app/(onboarding)/basic.tsx:74`
- `apps/mobile/src/app/(auth)/verify.tsx:128`

```typescript
// eslint-disable-next-line typescript/no-unsafe-return -- Elysia/Bun WS type mismatch
```

**Issue:** Multiple lint disables in the codebase. Some are legitimate (array index keys with fixed items) but the WebSocket type cast is concerning.

### 37. Font Files Contain HTML (Potential Corruption/Confusion)
**Files:** `apps/mobile/assets/fonts/*.ttf`

Grep shows font files contain HTML snippets from GitHub pages. This is likely font download corruption or bundling issue - fonts should only contain binary glyph data.

### 38. postMessage with Wildcard Origin
**File:** `apps/mobile/src/lib/web-phone-frame.ts:187`

```typescript
parent.postMessage({ type: 'navigation', path: pathname }, '*');
```

**Issue:** Using `'*'` as target origin means any parent window can receive these messages. Should specify the expected origin for security.

### 39. Complex Type Guards via globalThis Casting
**File:** `apps/mobile/src/lib/web-phone-frame.ts:46-76`

```typescript
function getWebDocument(): WebDocument | undefined {
	const doc = (globalThis as Record<string, unknown>)['document'];
	if (doc && typeof doc === 'object' && 'body' in doc && 'createElement' in doc && 'head' in doc) {
		return doc as WebDocument;
	}
	return undefined;
}
```

**Issue:** Multiple functions doing manual runtime type checking with casts. This is fragile and error-prone. Consider using a proper browser detection library or Expo's Platform API.

### 40. Default Email Domain is 'example.com'
**File:** `apps/mobile/src/lib/config.ts:56`

```typescript
return process.env['EXPO_PUBLIC_EMAIL_DOMAIN'] ?? 'example.com';
```

**Issue:** Falls back to 'example.com' which is a reserved domain. In production without env var set, users would register with invalid emails.

### 41. Inconsistent Null/Undefined Returns
**Files:** Various (auth-guard.ts, profiles/service.ts, etc.)

Functions inconsistently return `null` vs `undefined`:
- `return null;` - 15 occurrences
- `return undefined;` - 8 occurrences

**Issue:** Mix of null and undefined for "not found" semantics makes the codebase harder to reason about.

### 42. Underscore-Prefixed Unused Parameters
**File:** `apps/api/src/features/chats/websocket/handlers.ts:134, 147-148`

```typescript
function handleUnsubscribe(ws: WsConnection, _profileId: string, payload: unknown): Promise<void> {
// ...
function handlePing(_ws: WsConnection, _profileId: string, _payload: unknown): Promise<void> {
	sendWsMessage(_ws, 'pong', {});  // _ws is actually used!
```

**Issue:** `handlePing` uses `_ws` but the underscore prefix convention means "unused". Misleading naming.

### 43. Tag Hierarchy Lookup by ID or Name
**File:** `apps/mobile/src/hooks/use-tag-hierarchy.ts:38-39`

```typescript
const getChildrenForParent = (parent: TagType) =>
	childrenByParent[parent.id] ?? childrenByParent[parent.name] ?? [];
```

**Issue:** Falls back to looking up by name if ID lookup fails. This is fragile - what if a tag's name matches another tag's ID? Should use consistent keys.

### 44. Nullish Assignment Operator in Loop
**File:** `apps/mobile/src/hooks/use-tag-hierarchy.ts:29`

```typescript
(grouped[parentKey] ??= []).push(tag);
```

**Issue:** While valid, this one-liner is hard to read. The side effect (array creation) combined with mutation (push) in one expression is not obvious.

### 45. Weak Type Guards Only Check Structure, Not Types
**File:** `apps/mobile/src/hooks/chat/message-handlers.ts:15-21`

```typescript
function isConnectedPayload(p: unknown): p is ConnectedPayload {
	return typeof p === 'object' && p !== null;
}

function isSubscriptionPayload(p: unknown): p is SubscriptionPayload {
	return typeof p === 'object' && p !== null && 'conversationId' in p;
}
```

**Issue:** `isConnectedPayload` only checks if something is an object - any object passes. `isSubscriptionPayload` checks for key existence but not the value type. These could let malformed payloads through.

### 46. Duplicate Type Guard Pattern Across Files
**Files:**
- `apps/mobile/src/lib/web-phone-frame.ts:46-76`
- `apps/mobile/src/app/privacy.tsx:34-57`

Both files implement nearly identical manual type guards for web APIs (document, confirm, etc.) using globalThis casting.

**Issue:** Code duplication and fragile patterns repeated. Should be extracted to a shared web-utils module.

### 47. Custom @lintignore Comment (Non-Standard)
**Files:**
- `apps/mobile/src/lib/mention-parser.ts:14`
- `apps/mobile/src/hooks/api/use-chats.ts:204`

```typescript
/**
 * @lintignore
 */
```

**Issue:** `@lintignore` is not a standard JSDoc tag or lint directive. It appears to be a custom marker that doesn't actually disable any linting. Either remove it or use proper lint disable comments.

### 48. Date Math Without Timezone Consideration
**File:** `apps/mobile/src/lib/date.ts:22-48`

```typescript
export function formatRelativeTime(dateString: string): string {
	const date = new Date(dateString);
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
```

**Issue:** Uses local time difference. If server sends UTC timestamps and client is in a different timezone, relative times could be off. Should ensure both dates are in the same timezone before calculating.

### 49. Soft Delete Leaks Empty Content
**File:** `apps/api/src/features/chats/service/messages.ts:176-184`

```typescript
await db
	.update(messages)
	.set({
		content: '',
		contentIv: '',
		isDeleted: true,
		updatedAt: new Date(),
	})
```

**Issue:** Soft delete clears content but the empty strings are still transmitted. The mobile client checks `isDeleted` flag but the API response still includes `content: ''` and `contentIv: ''`. Consider not sending these fields at all for deleted messages.

### 50. Calendar Day Calculation Mutates Date Object
**File:** `apps/mobile/src/lib/calendar.ts:14-18`

```typescript
for (let i = 0; i < firstWeekday; i++) {
	const day = new Date(firstDayOfMonth);
	day.setDate(firstDayOfMonth.getDate() - (firstWeekday - i));
	days.push(day);
}
```

**Issue:** Creates new Date from `firstDayOfMonth` but then mutates it with `setDate()`. While this works because a new Date is created each iteration, the mutation pattern is easy to get wrong.

### 51. SQL Injection Risk in Tag Search
**File:** `apps/api/src/features/tags/service.ts:39`

```typescript
? await db.select().from(tags).where(and(eq(tags.scope, scope), ilike(tags.name, `%${query}%`))).limit(limit)
```

**Issue:** User input `query` is directly interpolated into the ILIKE pattern. While Drizzle should parameterize this, the pattern with `%` wildcards could allow users to craft slow queries (e.g., `%a%a%a%a%a%`). Consider escaping or limiting the query pattern.

### 52. Web Platform Uses localStorage for Auth Tokens
**File:** `apps/mobile/src/lib/storage.ts:16, 24`

```typescript
if (Platform.OS === 'web') {
	return localStorage.getItem(key);
// ...
if (Platform.OS === 'web') {
	localStorage.setItem(key, value);
```

**Issue:** On web, session tokens are stored in localStorage which is accessible to any JavaScript on the page (XSS risk). Native uses SecureStore. Consider using httpOnly cookies for web or sessionStorage at minimum.

### 53. Rate Limit Can Be Disabled via Environment
**File:** `apps/api/src/plugins/rate-limit.ts:14, 24`

```typescript
skip: () => !env.RATE_LIMIT_ENABLED,
```

**Issue:** Rate limiting can be entirely disabled via environment variable. If this accidentally gets set to false in production, the API has no rate limiting protection.

### 54. Token Prefix Logged (Potential Security Info Leak)
**File:** `apps/api/src/plugins/auth-guard.ts:68, 71`

```typescript
logger.info({ tokenPrefix: token.slice(0, 8) }, 'Auth: looking up session by token');
logger.info({ found: !!record, tokenPrefix: token.slice(0, 8) }, 'Auth: session lookup result');
```

**Issue:** Logging first 8 characters of tokens could help attackers if logs are compromised. Combined with timing attacks, this could leak information. Consider removing or using a hash instead.

### 55. In-Memory Token Cache Never Invalidated on Logout
**File:** `apps/mobile/src/lib/session-token.ts:5, 26`

```typescript
let inMemoryToken: string | null | undefined;
// ...
export async function setSessionToken(token: string | null): Promise<void> {
	inMemoryToken = token;
```

**Issue:** The `inMemoryToken` is a module-level variable. While `setSessionToken(null)` clears it, any code that imported the old value or cached `getSessionToken()` result may still have the old token. Not a major issue but could cause confusion.

### 56. Async Dynamic Import in Every Storage Operation
**File:** `apps/mobile/src/lib/storage.ts:6-11, 18, 27, 36`

```typescript
function getSecureStore() {
	if (Platform.OS === 'web') {
		return Promise.resolve(null);
	}
	return import('expo-secure-store');
}
// Called in every getItem, setItem, deleteItem
```

**Issue:** Every storage operation does a dynamic import of `expo-secure-store`. While module caching should help, this is unnecessary overhead. Import once at module level or cache the import result.

### 57. Toast setTimeout Not Cleaned Up on Unmount
**File:** `apps/mobile/src/lib/toast.tsx:27-31`

```typescript
addToast = (toast) => {
	const id = ++toastId;
	setToasts((prev) => [...prev, { ...toast, id }]);
	setTimeout(() => removeToast(id), TOAST_DURATION_MS);
};
```

**Issue:** The `setTimeout` inside `addToast` is not tracked or cleared. If the `ToastProvider` unmounts while a toast timeout is pending, it will try to call `setToasts` on an unmounted component. Low risk since ToastProvider typically lives at root level, but still a potential memory leak pattern.

### 58. Bookmark Cache Never Invalidated
**File:** `apps/mobile/src/lib/bookmark-store.ts:9-10, 46`

```typescript
let bookmarksCache: Set<string> | null = null;
// ...
bookmarksCache = bookmarks; // Set after storage write
```

**Issue:** The `bookmarksCache` module-level variable is never cleared between user sessions. If user A logs out and user B logs in, user B might see user A's cached bookmarks until app restart. Need to expose a cache clear function called on logout.

### 59. JSON.parse Without Try-Catch in Upload Error Path
**File:** `apps/mobile/src/hooks/api/use-upload.ts:70`

```typescript
const errorData = JSON.parse(uploadResult.body || '{}') as Record<string, unknown>;
```

**Issue:** If `uploadResult.body` is malformed JSON (not `null` but also not valid JSON), this will throw an unhandled exception. The `|| '{}'` fallback only handles empty/null case.

### 60. Tags Search Endpoint Has No Auth Check
**File:** `apps/api/src/features/tags/index.ts:16-26`

```typescript
.get(
	'/',
	async ({ query, db }) => {
		const { scope, search = '', limit = 20 } = query;
		return { data: await searchTags(db, scope, search, limit) };
	},
	// ... no requireAuth() call
)
```

**Issue:** The tags search endpoint doesn't require authentication. While tags themselves may be public, this allows unauthenticated users to probe the tag database and potentially enumerate tag names. Compare with POST endpoint which does call `requireAuth(user)`.

### 61. Upload GET Endpoint Public by Design (Documented Risk)
**File:** `apps/api/src/features/uploads/index.ts:20-22`

```typescript
// Serve uploaded files (public - UUIDs provide obscurity)
// Auth not required because <img> tags can't send headers
.get(
```

**Issue:** Already documented in comments, but worth noting: uploaded files are accessible to anyone who knows/guesses the UUID. Profile pictures, event covers, etc. are publicly accessible. This is intentional for `<img>` tag compatibility but means UUIDs must remain unguessable. Consider signed URLs or CDN with token validation.

### 62. HEIC/HEIF MIME Type Mapping May Be Incorrect
**File:** `apps/mobile/src/hooks/api/use-upload.ts:50-51`

```typescript
heic: 'image/jpeg', // HEIC gets converted by expo-image-picker with editing enabled
heif: 'image/jpeg',
```

**Issue:** Comment claims expo-image-picker converts HEIC to JPEG when editing is enabled, but the MIME type mapping still sends `image/jpeg` for `.heic` files. If editing is NOT enabled, the server receives a HEIC file labeled as JPEG. Server might accept it based on MIME type but fail to process it correctly.

### 63. Theme Store Returns Casted String Without Validation
**File:** `apps/mobile/src/lib/theme-store.ts:12-13`

```typescript
const theme = await storage.getItem(THEME_KEY);
return (theme as ThemeMode) || 'dark';
```

**Issue:** The stored value is cast to `ThemeMode` without validation. If storage is corrupted or contains an unexpected value (e.g., `"blue"`), the invalid value is returned and used. Should validate against allowed values.

### 64. Query Key Inconsistency - Profiles Invalidation
**Files:**
- `apps/mobile/src/hooks/auth/useRegisterForm.ts:65`
- `apps/mobile/src/hooks/auth/useLoginForm.ts:72`
- `apps/mobile/src/hooks/use-profile-creation.ts:34`

```typescript
// In useRegisterForm.ts and useLoginForm.ts:
queryClient.invalidateQueries({ queryKey: ['profiles'] });

// In use-profile-creation.ts:
queryClient.invalidateQueries({ queryKey: ['profiles', 'me'] });
```

**Issue:** Inconsistent query key invalidation. Some places invalidate `['profiles']` (broad), others invalidate `['profiles', 'me']` (specific). The broad invalidation might unnecessarily refetch all profile queries, while the specific one might miss some caches.

### 65. IIFE Pattern in useEffect Without Error Handling
**File:** `apps/mobile/src/app/settings.tsx:12-17`

```typescript
useEffect(() => {
	(async () => {
		const mode = await themeStore.getTheme();
		setIsDarkMode(mode === 'dark');
	})();
}, []);
```

**Issue:** The IIFE pattern for async in useEffect doesn't handle errors. If `themeStore.getTheme()` throws (unlikely but possible), the error is silently swallowed. Should add try-catch or use `.catch()`.

### 66. typingUsers State Can Grow Unboundedly
**File:** `apps/mobile/src/hooks/chat/useWebSocketEventHandlers.ts:28-30`

```typescript
if (payload.conversationId === conversationId && payload.profileId !== myProfileId) {
	setTypingUsers((prev) => new Set([...prev, payload.profileId]));
}
```

**Issue:** If `stop_typing` events are missed (network issues, server bugs), the `typingUsers` Set will grow indefinitely. Should add a timeout to auto-remove typing indicators after N seconds, or clear on component unmount.

### 67. extractData Requires Two Levels of .data Wrapping
**File:** `apps/mobile/src/lib/api-error.ts:64-66`

```typescript
const data = response.data;
if (data && typeof data === 'object' && hasDataProperty<T>(data)) {
	return data.data;
}
```

**Issue:** The `extractData` function expects `response.data.data` pattern (double-wrapped data). This is fragile and requires the API to always return this exact structure. If API response format changes, all call sites break silently by returning `undefined`.

### 68. Error Detection Based on String Matching
**File:** `apps/mobile/src/lib/auth-errors.ts:15-20, 35, 44, 53`

```typescript
return (
	msg.includes('already') ||
	msg.includes('exists') ||
	msg.includes('taken') ||
	error.code === 'USER_ALREADY_EXISTS'
);
// ... similar patterns for network, credentials, not found
```

**Issue:** Error detection relies on fragile string matching in error messages. If the server changes error message wording (e.g., "Email already exists" → "Email is in use"), the detection breaks. Should use error codes primarily, with message matching as fallback.

### 69. WebSocket Auto-Reconnect Has No Backoff/Max Retries
**File:** `apps/mobile/src/hooks/chat/useWebSocketConnection.ts:65`

```typescript
socket.onclose = () => {
	// ...
	reconnectTimeout.current = setTimeout(() => connectRef.current(), RECONNECT_DELAY);
};
```

**Issue:** WebSocket reconnection uses fixed 3s delay forever. If server is down or user has no internet, this will retry indefinitely every 3 seconds. Should implement exponential backoff and maximum retry count.

### 70. connectRef Pattern Is Unusual
**File:** `apps/mobile/src/hooks/chat/useWebSocketConnection.ts:108, 141`

```typescript
const connectRef = useRef<() => Promise<void>>(async () => {});
// ...
connectRef.current = connect;
```

**Issue:** The `connectRef` is initialized with an empty async function, then immediately overwritten with the actual `connect` function. This pattern exists to allow `onclose` handler to call `connect` without stale closure, but it's confusing. The brief window where `connectRef.current` is a no-op could cause issues if called during that time.

### 71. WebSocket Message Parse Errors Silently Ignored
**File:** `apps/mobile/src/hooks/chat/useWebSocketConnection.ts:51-57`

```typescript
socket.onmessage = (event) => {
	try {
		const message: WsMessage = JSON.parse(event.data);
		callbacksRef.current.onMessage(message);
	} catch (error) {
		logger.error('Failed to parse WebSocket message', error);
	}
};
```

**Issue:** If the server sends malformed JSON, the error is logged but the message is silently dropped. User has no indication that messages might be missing. Consider adding a callback for parse errors or showing a warning.

### 72. combineDateTime Mutates Input Date Object
**File:** `apps/mobile/src/lib/date-input.ts:35-45`

```typescript
export function combineDateTime(date: string, time: string): Date | null {
	const d = parseDate(date);
	if (!d) {
		return null;
	}
	// ...
	d.setHours(Number.parseInt(hh, 10) || 0, Number.parseInt(min, 10) || 0, 0, 0);
	return d;
}
```

**Issue:** While `parseDate` creates a new Date, the `setHours` call mutates it. This is fine since it's a locally created object, but the pattern of mutation could be error-prone if refactored. Also, `|| 0` silently converts invalid time parts to 0 instead of returning null.

### 73. Hooks Called with Empty String When No conversationId
**File:** `apps/mobile/src/components/event/detail/EventLiveChat.tsx:37-42`

```typescript
// Always call hooks - use empty string when no conversationId
const effectiveConversationId = conversationId ?? '';
const { data: messagesData } = useMessages(effectiveConversationId);
const sendMessage = useSendMessage(effectiveConversationId);
const addReaction = useAddReaction(effectiveConversationId);
const removeReaction = useRemoveReaction(effectiveConversationId);
```

**Issue:** Hooks are called with empty string `''` when there's no conversation. While `useMessages` has `enabled: !!conversationId`, the mutation hooks (`useSendMessage`, etc.) don't - they could potentially make API calls with empty conversation IDs. This relies on early returns in handlers but is fragile.

### 74. decryptMessage Called with Empty IV for Reply Preview
**File:** `apps/mobile/src/hooks/api/use-chats.ts:99`

```typescript
contentPreview: decryptMessage(msg.replyTo.contentPreview, ''),
```

**Issue:** The `decryptMessage` function is called with an empty string `''` for the IV when decrypting reply previews. The current placeholder crypto just does base64, so this works, but if real encryption is implemented, this will fail or produce undefined behavior.

### 75. betterAuthStorage.getItem Triggers Async Load Synchronously
**File:** `apps/mobile/src/lib/auth.ts:81-86`

```typescript
getItem(key: string): string | null {
	if (!storageCache.has(key) && !pendingLoads.has(key)) {
		void loadFromStorage(key);
	}
	return storageCache.get(key) ?? null;
}
```

**Issue:** The `getItem` is synchronous (required by Better Auth), but if the cache doesn't have the key, it triggers an async load and immediately returns `null`. This can cause race conditions where auth state appears unset on first read, then appears after storage loads. The `void` keyword shows the promise is intentionally not awaited.

### 76. clearAllAuthState Uses Hardcoded Storage Key Patterns
**File:** `apps/mobile/src/lib/auth.ts:189-194`

```typescript
const keysToDelete = [
	`${BETTER_AUTH_PREFIX}_session_token`,
	`${BETTER_AUTH_PREFIX}_session`,
	`${BETTER_AUTH_PREFIX}.session_token`,
	`${BETTER_AUTH_PREFIX}.session`,
];
```

**Issue:** Storage keys for Better Auth cleanup are hardcoded guesses (both underscore and dot separators). If Better Auth changes its internal key format, some keys may not be deleted, leaving stale auth state.

### 77. Reply Content Preview Uses Different Crypto Path
**File:** `apps/mobile/src/hooks/api/use-chats.ts:92-102`

```typescript
function decryptMessageContent(msg: MessageData): MessageData {
	return {
		...msg,
		content: msg.isDeleted ? '' : decryptMessage(msg.content, msg.contentIv),
		replyTo: msg.replyTo
			? {
					...msg.replyTo,
					contentPreview: decryptMessage(msg.replyTo.contentPreview, ''),
				}
			: null,
	};
}
```

**Issue:** Main message content uses `msg.contentIv`, but `replyTo.contentPreview` doesn't have an IV field - it's decrypted with empty string. This suggests the reply preview might be stored differently (truncated? pre-decrypted on server?). If server-side changes, this could break silently.

---

## Summary by Priority

### Must Fix (Security)
1. Implement real encryption (crypto.ts)
2. Remove Math.random fallback for crypto
3. Move WebSocket token out of URL
4. Validate upload file magic bytes, not just MIME type
5. Consider signed URLs for uploads instead of UUID obscurity
6. Fix postMessage wildcard origin (web-phone-frame.ts)
7. Strengthen WebSocket payload type guards (message-handlers.ts)
8. Don't use localStorage for auth tokens on web (XSS risk)
9. Escape/limit tag search query patterns (potential DoS)
10. Remove token prefix from logs

### High Priority (Core Functionality)
11. Implement real matching algorithm
12. Server-side bookmarks
13. Server-side search
14. Add missing chat management hooks
15. Set proper email domain default (not example.com)
16. Fix timezone handling in relative time formatting
17. Make rate limiting always-on in production

### Medium Priority (Code Quality)
18. Stop mutating objects with Object.assign
19. Extract complex components into smaller pieces
20. Consistent error message language
21. Use shared API client everywhere
22. Remove type assertions, use proper narrowing
23. Consistent null vs undefined semantics
24. Fix misleading underscore-prefixed params that ARE used
25. Extract duplicate web type guards to shared module
26. Don't send empty content fields for deleted messages
27. Cache dynamic import of expo-secure-store

### Low Priority (Cleanup)
28. Remove/implement TODO handlers
29. Fix naming inconsistencies
30. Address lint warnings
31. Investigate font file corruption
32. Simplify web-phone-frame type guards
33. Clean up complex one-liners (nullish assignment in loops)
34. Fix tag hierarchy ID/name lookup ambiguity
35. Remove non-standard @lintignore comments
36. Avoid Date mutation patterns in calendar
37. Clear bookmark cache on logout
38. Track/clear toast timeouts on unmount
39. Add try-catch around JSON.parse in upload error handling
40. Review HEIC MIME type mapping when editing is disabled
41. Validate theme values from storage
42. Standardize query key invalidation patterns
43. Add error handling to async IIFE in useEffects
44. Add typing indicator timeout to prevent unbounded growth
45. Document/simplify extractData double-wrapping pattern
46. Use error codes instead of message string matching
47. Add exponential backoff/max retries to WebSocket reconnect
48. Simplify connectRef pattern or add documentation
49. Consider surfacing WebSocket parse errors to user
50. Add `enabled` check to mutation hooks in EventLiveChat
51. Clarify reply preview encryption/IV handling
52. Document betterAuthStorage sync-over-async pattern
53. Use Better Auth's actual storage key format instead of guessing

---

*Generated: 2026-01-26*
*Updated: 2026-01-26 (iteration 9)*
*Total issues cataloged: 77*
*Total files analyzed: ~190*
*Warnings from oxlint: 129*
*Issues from ast-grep: 8*
