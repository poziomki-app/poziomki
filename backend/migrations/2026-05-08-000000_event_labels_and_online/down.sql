DROP FUNCTION IF EXISTS app.admin_set_event_labels(uuid, text[]);

DROP INDEX IF EXISTS idx_events_labels_gin;

ALTER TABLE public.events
    DROP COLUMN IF EXISTS meeting_url,
    DROP COLUMN IF EXISTS is_online,
    DROP COLUMN IF EXISTS labels;
