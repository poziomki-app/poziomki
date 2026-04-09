-- Remove conversations whose linked event no longer exists.
-- These were orphaned by event deletions that didn't clean up conversations.
DELETE FROM conversations
WHERE event_id IS NOT NULL
  AND event_id NOT IN (SELECT id FROM events);
