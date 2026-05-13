-- Event labels (admin-curated, e.g. 'featured') + online-meeting fields.
-- Labels are a free-form TEXT[] so adding a new badge value is a code-only
-- change in the admin endpoint allowlist; no enum migration required.
ALTER TABLE public.events
    ADD COLUMN IF NOT EXISTS labels TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS is_online BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS meeting_url VARCHAR;

-- Partial index keeps "featured" lookups cheap without bloating the rest.
CREATE INDEX IF NOT EXISTS idx_events_labels_gin
    ON public.events USING GIN (labels)
    WHERE labels <> '{}';

-- Mirrors app.admin_ban_user — admin endpoint has no viewer context, so
-- bypass RLS via SECURITY DEFINER. Returns true when the row exists.
CREATE OR REPLACE FUNCTION app.admin_set_event_labels(p_event_id uuid, p_labels text[])
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    v_found boolean;
BEGIN
    UPDATE public.events
    SET labels = COALESCE(p_labels, '{}'),
        updated_at = now()
    WHERE id = p_event_id
    RETURNING TRUE INTO v_found;

    RETURN COALESCE(v_found, FALSE);
END
$$;

COMMENT ON FUNCTION app.admin_set_event_labels(uuid, text[]) IS
    'Admin-only: replace the labels array on an event. Called by /api/v1/admin/events/{id}/labels which has no viewer context.';

REVOKE EXECUTE ON FUNCTION app.admin_set_event_labels(uuid, text[]) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.admin_set_event_labels(uuid, text[]) TO poziomki_api';
    END IF;
END
$$;
