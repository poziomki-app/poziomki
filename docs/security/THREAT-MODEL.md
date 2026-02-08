# Threat Model - Poziomki

**Document Owner:** Security Team
**Last Updated:** 2026-02-02
**Review Frequency:** Quarterly

---

## 1. System Overview

### Application Description

Poziomki is a social mobile application for university students that enables:
- Profile creation and browsing
- Interest-based matching
- Direct messaging (chat)
- Event creation and attendance
- Photo sharing

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        EXTERNAL ZONE                             │
│  ┌──────────────┐                                                │
│  │ Mobile App   │  (iOS/Android - User devices)                  │
│  │ (Expo/RN)    │                                                │
│  └──────┬───────┘                                                │
│         │ HTTPS/WSS                                              │
├─────────┼───────────────────────────────────────────────────────┤
│         │              DMZ / EDGE                                │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │    Caddy     │  (Reverse proxy, TLS termination)              │
│  │   (HTTPS)    │                                                │
│  └──────┬───────┘                                                │
│         │                                                        │
├─────────┼───────────────────────────────────────────────────────┤
│         │           APPLICATION ZONE                             │
│         ▼                                                        │
│  ┌──────────────┐     ┌──────────────┐                          │
│  │  Elysia API  │────▶│   MinIO      │  (Object storage)         │
│  │   (Bun)      │     │  (S3-compat) │                          │
│  └──────┬───────┘     └──────────────┘                          │
│         │                                                        │
├─────────┼───────────────────────────────────────────────────────┤
│         │              DATA ZONE                                 │
│         ▼                                                        │
│  ┌──────────────┐                                                │
│  │ PostgreSQL   │  (Primary database)                            │
│  │    17        │                                                │
│  └──────────────┘                                                │
└─────────────────────────────────────────────────────────────────┘
```

### Trust Boundaries

| Boundary | From | To | Protection |
|----------|------|----|------------|
| **B1** | Internet → Caddy | Untrusted → Edge | TLS, rate limiting |
| **B2** | Caddy → API | Edge → Application | Internal network |
| **B3** | API → Database | Application → Data | Credentials, SQL parameterization |
| **B4** | API → MinIO | Application → Storage | Credentials, presigned URLs |
| **B5** | Mobile → CDN | Client → Storage | Auth check, presigned URLs |

---

## 2. Assets & Data Classification

### Data Assets

| Asset | Classification | Description |
|-------|----------------|-------------|
| **User credentials** | Critical | Passwords (hashed), session tokens |
| **Personal identifiable info** | High | Email, name, profile photos, bio |
| **Chat messages** | High | Private communications |
| **Location data** | High | Event locations, user-provided locations |
| **Session tokens** | High | Authentication state |
| **Event data** | Medium | Public event information |
| **System credentials** | Critical | API keys, database passwords |
| **Audit logs** | Medium | Security and access logs |

### System Assets

| Asset | Criticality | Description |
|-------|-------------|-------------|
| **API Server** | Critical | Core application logic |
| **PostgreSQL** | Critical | All persistent data |
| **MinIO** | High | User-uploaded media |
| **Caddy** | High | Entry point, TLS termination |
| **Backups** | High | Data recovery capability |

---

## 3. Threat Actors

| Actor | Capability | Motivation | Likely Targets |
|-------|------------|------------|----------------|
| **Script Kiddie** | Low | Curiosity, bragging | Public endpoints, known CVEs |
| **Opportunistic Attacker** | Medium | Data theft, credentials | User accounts, exposed APIs |
| **Targeted Attacker** | High | Specific user data | Individual accounts, personal data |
| **Insider Threat** | High | Various | Any system with access |
| **Competitor** | Medium | Business intelligence | User data, feature analysis |
| **Nation State** | Very High | Surveillance | Unlikely for this app |

### Attack Surface

| Entry Point | Exposure | Protection |
|-------------|----------|------------|
| HTTPS API endpoints | Public | Rate limiting, authentication |
| WebSocket connections | Public | Token auth, rate limiting |
| CDN file access | Public | Auth check, presigned URLs |
| Admin endpoints | Private | Not implemented (future risk) |
| Database | Private | Network isolation, credentials |
| MinIO | Private | Network isolation, credentials |

---

## 4. STRIDE Analysis

### Spoofing

| Threat | Component | Risk | Mitigation |
|--------|-----------|------|------------|
| Impersonate another user | API Auth | High | Session tokens, secure cookies |
| Forge session token | API Auth | Critical | Cryptographically secure tokens |
| Spoof email verification | Auth flow | High | OTP with expiration |
| Impersonate API server | Mobile app | Medium | Certificate pinning (TODO) |

### Tampering

| Threat | Component | Risk | Mitigation |
|--------|-----------|------|------------|
| Modify messages in transit | Network | Low | TLS encryption |
| Modify stored messages | Database | Medium | Access controls, audit logging |
| Tamper with uploads | MinIO | Medium | Checksums, access controls |
| Modify API requests | Client | Medium | Input validation, server-side checks |

### Repudiation

| Threat | Component | Risk | Mitigation |
|--------|-----------|------|------------|
| Deny sending message | Chat | Medium | Message storage with timestamps |
| Deny account actions | All | Medium | Audit logging |
| Deny accessing data | All | Medium | Request logging |

### Information Disclosure

| Threat | Component | Risk | Mitigation |
|--------|-----------|------|------------|
| Expose user credentials | Database | Critical | Hashing, encryption at rest |
| Leak chat messages | API/DB | High | Access controls, encryption (TODO) |
| Expose session tokens | Logs/URLs | High | Token redaction, header auth |
| Profile enumeration | API | Medium | Constant-time responses (TODO) |
| File access bypass | MinIO | High | Auth check before CDN access |

### Denial of Service

| Threat | Component | Risk | Mitigation |
|--------|-----------|------|------------|
| API flooding | API | Medium | Rate limiting |
| WebSocket flooding | Chat | Medium | Per-connection limits (TODO) |
| Database exhaustion | PostgreSQL | Medium | Connection pooling, query limits |
| Storage exhaustion | MinIO | Medium | Upload size limits, quotas |

### Elevation of Privilege

| Threat | Component | Risk | Mitigation |
|--------|-----------|------|------------|
| Access other user's data | API | High | Authorization checks |
| Delete other user's files | MinIO | Critical | Ownership validation (TODO) |
| Access event chat without attendance | Chat | High | Attendance check (TODO) |
| Bypass rate limits | API | Medium | Account-level limits (TODO) |

---

## 5. Data Flow Analysis

### Authentication Flow

```
Mobile App                    API                      Database
    │                          │                          │
    │─── POST /auth/email ────▶│                          │
    │    {email}               │─── Check user exists ───▶│
    │                          │◀── User record ──────────│
    │                          │─── Generate OTP ─────────│
    │                          │─── Send email ───────────│
    │◀── {pending} ────────────│                          │
    │                          │                          │
    │─── POST /auth/verify ───▶│                          │
    │    {email, otp}          │─── Verify OTP ──────────▶│
    │                          │◀── Valid ────────────────│
    │                          │─── Create session ──────▶│
    │◀── {token} ──────────────│                          │
    │                          │                          │
```

**Threats:**
- OTP brute force → Mitigated by rate limiting, expiration
- Email enumeration → Partially mitigated (constant responses TODO)
- Token theft → Mitigated by HTTPS, secure storage

### Message Flow

```
Sender App         API/WebSocket         Database         Receiver App
    │                   │                    │                  │
    │─── WS message ───▶│                    │                  │
    │    {content}      │─── Store message ─▶│                  │
    │                   │◀── Stored ─────────│                  │
    │                   │─── Get recipients ─▶│                  │
    │                   │◀── Participant IDs ─│                  │
    │                   │─── Broadcast ───────────────────────▶│
    │◀── ACK ───────────│                    │                  │
```

**Threats:**
- Message interception → Mitigated by TLS (E2E encryption TODO)
- Unauthorized chat access → Participant check required
- Message spam → Rate limiting TODO

### File Upload Flow

```
Mobile App              API                 MinIO             CDN/Caddy
    │                    │                    │                   │
    │─── Upload file ───▶│                    │                   │
    │    (multipart)     │─── Validate ───────│                   │
    │                    │─── Store file ────▶│                   │
    │                    │◀── Stored ─────────│                   │
    │                    │─── Record in DB ───│                   │
    │◀── {url} ──────────│                    │                   │
    │                    │                    │                   │
    │───────────────────────────────────────────── Request file ─▶│
    │                    │◀── Auth check ─────────────────────────│
    │                    │─── Check access ───│                   │
    │                    │─── Allow/Deny ─────────────────────────▶│
    │◀─────────────────────────────────────────────── File ───────│
```

**Threats:**
- Malicious file upload → Magic byte validation
- Unauthorized file access → Auth check required
- File enumeration → Random UUIDs for filenames
- EXIF data leakage → Strip metadata TODO

---

## 6. Risk Matrix

### Likelihood × Impact Scoring

| | Negligible (1) | Minor (2) | Moderate (3) | Significant (4) | Severe (5) |
|---|---|---|---|---|---|
| **Almost Certain (5)** | 5 | 10 | 15 | 20 | 25 |
| **Likely (4)** | 4 | 8 | 12 | 16 | 20 |
| **Possible (3)** | 3 | 6 | 9 | 12 | 15 |
| **Unlikely (2)** | 2 | 4 | 6 | 8 | 10 |
| **Rare (1)** | 1 | 2 | 3 | 4 | 5 |

### Risk Levels

| Score | Level | Response |
|-------|-------|----------|
| 15-25 | Critical | Immediate action required |
| 10-14 | High | Address within 1 week |
| 5-9 | Medium | Address within 1 month |
| 1-4 | Low | Accept or address opportunistically |

### Top Risks

| # | Threat | L | I | Score | Status |
|---|--------|---|---|-------|--------|
| 1 | Messages not encrypted (base64 only) | 5 | 5 | **25** | CRITICAL - Fix required |
| 2 | File deletion IDOR | 4 | 5 | **20** | CRITICAL - Fix required |
| 3 | Event chat join without attendance | 4 | 4 | **16** | HIGH - Fix required |
| 4 | WebSocket token in URL | 4 | 4 | **16** | HIGH - Fix required |
| 5 | No certificate pinning | 3 | 4 | **12** | HIGH - Implement |
| 6 | Missing account lockout | 4 | 3 | **12** | HIGH - Implement |
| 7 | Profile enumeration | 3 | 3 | **9** | MEDIUM |
| 8 | No WebSocket rate limiting | 3 | 3 | **9** | MEDIUM |

---

## 7. Attack Chains

### Chain 1: Account Takeover

```
1. Attacker identifies target email (public profile, guessing)
        ↓
2. Attacker triggers OTP to target email
        ↓
3. Attacker brute-forces OTP (6 digits, no lockout)
        ↓
4. Attacker gains session token
        ↓
5. Attacker accesses all profile data, messages, events
```

**Mitigations needed:**
- [ ] OTP attempt limiting per email
- [ ] Account lockout after N failures
- [ ] Notification to user on login

### Chain 2: Data Exfiltration via File Access

```
1. Attacker creates account (minimal verification)
        ↓
2. Attacker discovers file naming pattern (UUIDs)
        ↓
3. Attacker enumerates files (backward compatibility bypass)
        ↓
4. Attacker accesses untracked files without ownership check
        ↓
5. Attacker downloads other users' private photos
```

**Mitigations needed:**
- [ ] Complete file backfill migration
- [ ] Remove backward compatibility bypass
- [ ] Add ownership validation for all file operations

### Chain 3: Conversation Infiltration

```
1. Attacker finds public event they're not attending
        ↓
2. Attacker calls getOrCreateEventChat(eventId)
        ↓
3. API auto-adds attacker as participant (no attendance check)
        ↓
4. Attacker reads private event chat history
        ↓
5. Attacker can impersonate attendee in chat
```

**Mitigations needed:**
- [ ] Verify event attendance before chat access
- [ ] Audit existing unauthorized participants

---

## 8. Security Controls Summary

### Implemented Controls

| Control | Status | Notes |
|---------|--------|-------|
| TLS encryption | ✅ | Caddy auto HTTPS |
| Password hashing | ✅ | Using secure algorithm |
| Session tokens | ✅ | Cryptographically secure |
| Rate limiting | ✅ | Per-IP, auth endpoints |
| Input validation | ✅ | Schema validation |
| SQL injection prevention | ✅ | Drizzle ORM |
| File type validation | ✅ | Magic bytes + MIME |
| Token redaction in logs | ✅ | Sensitive data scrubbed |
| CORS configuration | ✅ | Configured for domains |

### Missing Controls

| Control | Priority | Effort |
|---------|----------|--------|
| E2E message encryption | Critical | High |
| File ownership validation | Critical | Low |
| Event attendance for chat | Critical | Low |
| Certificate pinning | High | Medium |
| Account lockout | High | Low |
| WebSocket rate limiting | High | Medium |
| EXIF stripping | High | Low |
| User enumeration prevention | Medium | Medium |

---

## 9. Recommendations

### Immediate (P0)

1. **Implement real encryption for messages** — Current base64 encoding provides zero security
2. **Add file ownership validation** — Delete endpoint has IDOR vulnerability
3. **Add attendance check for event chats** — Privacy violation
4. **Move WebSocket token from URL to header** — Tokens visible in logs

### Short-term (P1)

5. **Implement certificate pinning** — Prevent MITM on mobile
6. **Add account lockout** — Prevent brute force
7. **Add WebSocket rate limiting** — Prevent flooding
8. **Strip EXIF from uploads** — Privacy protection
9. **Remove backward compatibility file bypass** — Complete migration first

### Medium-term (P2)

10. **Implement user enumeration prevention** — Constant-time responses
11. **Add session binding** — Device/location validation
12. **Implement audit logging** — For compliance and forensics
13. **Add content moderation** — Reporting and blocking

---

## 10. Review Schedule

| Activity | Frequency | Next Review |
|----------|-----------|-------------|
| Threat model review | Quarterly | 2026-05-01 |
| Risk assessment update | Monthly | 2026-03-01 |
| Penetration testing | Annually | Before public launch |
| Security audit | Annually | Before public launch |

---

## 11. Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-02-02 | Security Team | Initial threat model |
