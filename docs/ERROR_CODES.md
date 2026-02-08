# API Error Codes

Reference for all error codes returned by the Poziomki API.

**API Documentation:** `/api/docs` (interactive) or `/api/docs/json` (OpenAPI spec)

---

## Response Format

All errors follow a consistent JSON structure:

```json
{
  "error": "Human-readable error message",
  "code": "ERROR_CODE",
  "requestId": "uuid-for-support",
  "details": "optional additional context"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `error` | string | Human-readable message (may be shown to users) |
| `code` | string | Machine-readable error code (use for client logic) |
| `requestId` | string | UUID for tracing/support requests |
| `details` | any | Optional additional context (validation errors, etc.) |

---

## Error Codes by HTTP Status

### 400 Bad Request

| Code | Description | Typical Causes |
|------|-------------|----------------|
| `VALIDATION_ERROR` | Input validation failed | Missing required fields, invalid email format, password too short/long, invalid field values |
| `BAD_REQUEST` | Invalid operation | Creating chat with self, editing deleted message, invalid emoji |
| `INVALID_FILE_TYPE` | File type not allowed | Only JPEG, PNG, WebP, AVIF images accepted |
| `FILE_TOO_LARGE` | File exceeds size limit | Maximum 10 MB per file |
| `INVALID_FILE_CONTENT` | File content doesn't match type | Magic bytes validation failed (file header doesn't match extension) |
| `INVALID_FILENAME` | Dangerous filename | Directory traversal characters detected (`../`, etc.) |
| `INVALID_URI` | CDN auth check failed | Malformed X-Original-URI header (internal) |
| `MISSING_URI` | CDN auth check failed | Missing X-Original-URI header (internal) |
| `INVALID_DATE_RANGE` | Invalid event dates | Event end time is before start time |

### 401 Unauthorized

| Code | Description | Typical Causes |
|------|-------------|----------------|
| `UNAUTHORIZED` | Authentication required or failed | Missing/invalid session token, wrong password |

### 403 Forbidden

| Code | Description | Typical Causes |
|------|-------------|----------------|
| `FORBIDDEN` | Permission denied | Accessing another user's resource, not a chat participant, not event creator |
| `ACCESS_DENIED` | File access denied | User cannot access this file |

### 404 Not Found

| Code | Description | Typical Causes |
|------|-------------|----------------|
| `NOT_FOUND` | Resource not found | Profile, event, conversation, message, or file doesn't exist |
| `PROFILE_NOT_FOUND` | Profile doesn't exist | Specific case during onboarding/upload |

### 409 Conflict

| Code | Description | Typical Causes |
|------|-------------|----------------|
| `CONFLICT` | Resource already exists | Profile already created, duplicate tag |

### 429 Too Many Requests

| Code | Description | Typical Causes |
|------|-------------|----------------|
| `RATE_LIMIT_EXCEEDED` | Rate limit hit | Too many requests in time window |

**Rate Limits:**
- Auth endpoints (`/auth/*`): 5 requests/minute
- General endpoints: 100 requests/minute

### 500 Internal Server Error

| Code | Description | Typical Causes |
|------|-------------|----------------|
| `INTERNAL_ERROR` | Server error | Database failure, unexpected condition (report with requestId) |

---

## Error Codes by Feature

### Authentication (`/auth/*`)

| Endpoint | Possible Codes |
|----------|----------------|
| `POST /auth/email` | `VALIDATION_ERROR`, `RATE_LIMIT_EXCEEDED` |
| `POST /auth/verify` | `VALIDATION_ERROR`, `NOT_FOUND`, `UNAUTHORIZED` |
| `POST /auth/register` | `VALIDATION_ERROR`, `CONFLICT` |
| `POST /auth/signin` | `VALIDATION_ERROR`, `NOT_FOUND`, `UNAUTHORIZED` |
| `POST /auth/signout` | `UNAUTHORIZED` |
| `DELETE /auth/account` | `UNAUTHORIZED`, `INTERNAL_ERROR` |

### Profiles (`/profiles/*`)

| Endpoint | Possible Codes |
|----------|----------------|
| `GET /profiles/:id` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |
| `POST /profiles` | `UNAUTHORIZED`, `VALIDATION_ERROR`, `CONFLICT` |
| `PATCH /profiles/:id` | `UNAUTHORIZED`, `VALIDATION_ERROR`, `NOT_FOUND`, `FORBIDDEN` |
| `DELETE /profiles/:id` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |

### Events (`/events/*`)

| Endpoint | Possible Codes |
|----------|----------------|
| `GET /events/:id` | `UNAUTHORIZED`, `NOT_FOUND` |
| `POST /events` | `UNAUTHORIZED`, `VALIDATION_ERROR`, `INVALID_DATE_RANGE` |
| `PATCH /events/:id` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |
| `DELETE /events/:id` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |
| `POST /events/:id/join` | `UNAUTHORIZED`, `NOT_FOUND`, `CONFLICT` |
| `POST /events/:id/leave` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |

### Chats (`/chats/*`)

| Endpoint | Possible Codes |
|----------|----------------|
| `GET /chats` | `UNAUTHORIZED` |
| `GET /chats/:id` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |
| `POST /chats/personal` | `UNAUTHORIZED`, `BAD_REQUEST`, `NOT_FOUND` |
| `POST /chats/:id/messages` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |
| `PATCH /chats/:id/messages/:msgId` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN`, `BAD_REQUEST` |
| `DELETE /chats/:id/messages/:msgId` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |

### Uploads (`/uploads/*`)

| Endpoint | Possible Codes |
|----------|----------------|
| `POST /uploads` | `UNAUTHORIZED`, `VALIDATION_ERROR`, `INVALID_FILE_TYPE`, `FILE_TOO_LARGE`, `INVALID_FILE_CONTENT`, `INVALID_FILENAME` |
| `DELETE /uploads/:id` | `UNAUTHORIZED`, `NOT_FOUND`, `FORBIDDEN` |
| `GET /cdn-auth` | `MISSING_URI`, `INVALID_URI`, `ACCESS_DENIED`, `NOT_FOUND` |

### Matching (`/matching/*`)

| Endpoint | Possible Codes |
|----------|----------------|
| `GET /matching/recommendations` | `UNAUTHORIZED`, `INTERNAL_ERROR` |

---

## Client-Side Handling

### TypeScript Example

```typescript
type ErrorCode =
  | 'VALIDATION_ERROR'
  | 'BAD_REQUEST'
  | 'INVALID_FILE_TYPE'
  | 'FILE_TOO_LARGE'
  | 'INVALID_FILE_CONTENT'
  | 'INVALID_FILENAME'
  | 'INVALID_DATE_RANGE'
  | 'UNAUTHORIZED'
  | 'FORBIDDEN'
  | 'ACCESS_DENIED'
  | 'NOT_FOUND'
  | 'PROFILE_NOT_FOUND'
  | 'CONFLICT'
  | 'RATE_LIMIT_EXCEEDED'
  | 'INTERNAL_ERROR';

interface ApiError {
  error: string;
  code: ErrorCode;
  requestId: string;
  details?: unknown;
}

function handleApiError(error: ApiError): string {
  switch (error.code) {
    case 'VALIDATION_ERROR':
      return error.error; // Show validation message to user
    case 'UNAUTHORIZED':
      // Redirect to login
      return 'Please sign in again';
    case 'FORBIDDEN':
      return 'You don\'t have permission to do this';
    case 'NOT_FOUND':
      return 'Not found';
    case 'RATE_LIMIT_EXCEEDED':
      return 'Too many requests. Please wait a moment.';
    case 'CONFLICT':
      return error.error; // Usually informative
    case 'INTERNAL_ERROR':
      // Log requestId for support
      console.error('Server error:', error.requestId);
      return 'Something went wrong. Please try again.';
    default:
      return 'An error occurred';
  }
}
```

### User-Facing vs Technical Errors

| Code | Show to User? | Notes |
|------|---------------|-------|
| `VALIDATION_ERROR` | Yes | Error message is user-friendly |
| `UNAUTHORIZED` | Yes | "Please sign in" |
| `FORBIDDEN` | Yes | "You don't have permission" |
| `NOT_FOUND` | Depends | May indicate stale data |
| `CONFLICT` | Yes | Usually informative |
| `RATE_LIMIT_EXCEEDED` | Yes | Ask user to wait |
| `INTERNAL_ERROR` | Generic only | Log requestId, show generic message |
| File errors | Yes | Error messages are user-friendly |

---

## Reporting Issues

When reporting API issues, include:
1. **requestId** from the error response
2. **Endpoint** called (method + path)
3. **Request body** (without sensitive data)
4. **Timestamp** of the error

---

*Last updated: 2026-02-03*
