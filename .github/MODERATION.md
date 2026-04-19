# Content moderation policy

This document describes how Poziomki handles user-generated content
during the invite-only beta. It is the public contract that stands in
for automated CSAM / abuse scanning (which will be added before public
launch).

## Scope

User-generated content on Poziomki:

- **Profile content** — display name, bio, profile pictures, gallery,
  interest tags
- **Event content** — event titles, descriptions, cover images,
  location text
- **Chat content** — DM messages, event chat messages, attachments,
  reactions

## Prohibited content

The following is not allowed and will result in account termination
at the sole discretion of the Poziomki team:

- Sexual content involving minors (any form, any context)
- Non-consensual intimate imagery
- Threats of violence, doxxing, or targeted harassment
- Impersonation of identifiable real people without consent
- Content violating Polish or EU law

## Enforcement

During the invite-only beta, moderation is manual:

1. **User reports** route to the `reports` table in the database.
   Any authenticated user can report a profile, an event, or a
   conversation.
2. **Admin review** — the Poziomki team reviews reports on a rolling
   basis. There is no SLA during beta; critical reports (e.g. CSAM)
   should be emailed to `moderation@poziomki.app` in parallel so they
   don't wait on the queue.
3. **Actions available to admins**:
   - Soft ban (`POST /api/v1/admin/users/:pid/ban`) — flips
     `users.banned_at`, invalidates all sessions, and rejects future
     auth attempts with a 403 + `ACCOUNT_BANNED` code.
   - Hard delete — cascaded account purge (same path used by
     user-initiated deletion).
   - Content removal — direct DB action for specific rows (chat
     messages, uploads, reports).

## Automation roadmap (GA blocker, not beta blocker)

Before Poziomki opens to public signups the following must land:

- [ ] Image hashing against a known-CSAM hash list (evaluation of free
      options like NCMEC hash distribution for qualifying orgs, or
      open-source alternatives)
- [ ] Toxicity classifier on message content (Perspective API has a
      free tier with rate limits; self-hosted alternatives exist)
- [ ] Automated action pipeline — hash match → auto-suspend + admin
      review, rather than manual-only
- [ ] Appeals process — banned users can request review via an email
      that doesn't require an active account

## Data retention

- `audit.events` retains account-state changes (ban / unban) indefinitely
  for forensic review
- Reports retain reporter identity for 90 days after the report is
  resolved, then `reporter_id` is anonymised
- Hard-deleted accounts purge all user-identifiable rows; only
  `audit.events` entries mentioning the internal `actor_user_id`
  survive
