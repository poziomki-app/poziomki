DROP FUNCTION IF EXISTS app.admin_set_event_featured(uuid, boolean);
DROP INDEX IF EXISTS events_is_featured_idx;
ALTER TABLE public.events DROP COLUMN is_featured;
