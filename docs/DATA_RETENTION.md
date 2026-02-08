# Data Retention Policy

How long we keep different types of data and why.

**Related:** [Privacy Policy](./PRIVACY_POLICY.md)

---

## Overview

| Principle | Policy |
|-----------|--------|
| **Minimization** | Collect only what's necessary |
| **Purpose limitation** | Use data only for stated purposes |
| **Storage limitation** | Delete when no longer needed |
| **Transparency** | Users know what we keep and why |

---

## Retention Schedule

### User Account Data

| Data Type | Retention | Deletion Trigger | Rationale |
|-----------|-----------|------------------|-----------|
| Email | Until deletion | Account deletion | Required for login |
| Password hash | Until deletion | Account deletion | Required for auth |
| Display name | Until deletion | Account deletion | Core profile data |
| Session tokens | 30 days | Logout or expiry | Security |

### Profile Data

| Data Type | Retention | Deletion Trigger | Rationale |
|-----------|-----------|------------------|-----------|
| Bio | Until deletion | Account deletion or user edit | User-controlled |
| Interests/tags | Until deletion | Account deletion or user edit | User-controlled |
| Profile photo | Until deletion | Account deletion or user edit | User-controlled |
| Study program | Until deletion | Account deletion or user edit | User-controlled |

### Content

| Data Type | Retention | Deletion Trigger | Rationale |
|-----------|-----------|------------------|-----------|
| Chat messages | Until deleted | Conversation deletion or account deletion | Communication record |
| Event data | 1 year after event | Automatic cleanup | Historical reference |
| Event chat | Until event deletion | Event cleanup | Associated content |
| Uploaded photos | Until deleted | User deletion or account deletion | User-controlled |

### Technical Data

| Data Type | Retention | Deletion Trigger | Rationale |
|-----------|-----------|------------------|-----------|
| Request logs | 30 days | Automatic rotation | Security, debugging |
| Error logs | 30 days | Automatic rotation | Debugging |
| Security logs | 90 days | Automatic rotation | Incident investigation |
| Rate limit data | 24 hours | Automatic expiry | Abuse prevention |

### Backups

| Data Type | Retention | Deletion Trigger | Rationale |
|-----------|-----------|------------------|-----------|
| Database backups | 30 days | Rotation | Disaster recovery |
| File storage backups | 30 days | Rotation | Disaster recovery |

---

## Account Deletion

When a user deletes their account:

### Immediately Deleted
- Profile information (name, bio, photo, interests)
- Account credentials
- Active sessions
- Preference settings

### Anonymized (Not Deleted)
- Chat messages — Content retained for other participants, sender anonymized to "Deleted User"
- Event attendance records — Anonymized for event statistics
- Created events — Transferred to "Unknown Organizer" or deleted if no attendees

### Retained for Legal/Security
- Security logs (90 days) — Required for incident investigation
- Abuse reports involving the user (1 year) — Required for platform safety

### Backup Purge
- User data removed from backups within 30 days
- Full deletion confirmed after backup rotation completes

---

## Data Export (GDPR Article 20)

Users can export their data in machine-readable format:

### Included in Export
- Account information (email, name)
- Profile data (bio, interests, photos)
- Messages sent
- Events created
- Events attended

### Export Format
```
export/
├── account.json       # Account info
├── profile.json       # Profile data
├── messages/          # Chat history
│   └── conversation-{id}.json
├── events/            # Events created
│   └── event-{id}.json
└── photos/            # Uploaded photos
    └── photo-{id}.jpg
```

### How to Request
Settings > Privacy > Download my data

Processing time: Up to 48 hours for large exports.

---

## Automated Cleanup Jobs

| Job | Schedule | Action |
|-----|----------|--------|
| Session cleanup | Daily | Delete expired sessions |
| Log rotation | Daily | Archive and delete old logs |
| Rate limit cleanup | Hourly | Clear expired rate limit entries |
| Event cleanup | Weekly | Archive events older than 1 year |
| Backup rotation | Daily | Delete backups older than 30 days |
| Orphaned files | Weekly | Delete unlinked uploaded files |

---

## Legal Holds

In case of legal proceedings or investigations:
- Relevant data may be preserved beyond normal retention
- Users are notified unless legally prohibited
- Data is deleted after hold is lifted

---

## Exceptions

### Abuse Prevention
Data related to policy violations may be retained longer to prevent repeat offenses:
- Banned email addresses (hashed): Indefinitely
- Abuse reports: 1 year after resolution
- IP addresses associated with abuse: 1 year

### Legal Requirements
Some data may be retained longer if required by:
- Court orders
- Regulatory requirements
- Ongoing investigations

---

## Implementation

### Database
```sql
-- Example: Automatic event cleanup
DELETE FROM events
WHERE ends_at < NOW() - INTERVAL '1 year';

-- Example: Log rotation
DELETE FROM request_logs
WHERE created_at < NOW() - INTERVAL '30 days';
```

### Scheduled Tasks
Cleanup jobs run via cron or similar scheduler. See deployment configuration for details.

### Verification
- Monthly audit of retention compliance
- Automated alerts for cleanup job failures
- Quarterly review of retention periods

---

## User Controls

| Action | How | Result |
|--------|-----|--------|
| Delete message | Long-press > Delete | Message removed |
| Delete photo | Profile > Edit > Remove | Photo deleted |
| Clear chat | Chat > Settings > Delete | Conversation cleared |
| Delete account | Settings > Account > Delete | Full account deletion |
| Export data | Settings > Privacy > Download | Data export generated |

---

## Changes to This Policy

Retention periods may be adjusted. Significant changes will be communicated via:
- In-app notification
- Privacy Policy update
- Email (for material changes)

---

## Questions

**Privacy inquiries:** privacy@poziomki.app
**Data deletion requests:** privacy@poziomki.app
**Legal inquiries:** legal@poziomki.app

---

*This policy implements GDPR Article 5(1)(e) - Storage Limitation Principle.*

*Last updated: 2026-02-03*
