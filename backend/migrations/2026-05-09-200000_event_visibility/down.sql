DROP INDEX IF EXISTS events_visibility_idx;
ALTER TABLE public.events DROP COLUMN IF EXISTS visibility;
