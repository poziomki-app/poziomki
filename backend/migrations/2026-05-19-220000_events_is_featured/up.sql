-- Featured events: admin-only flag that floats an event to the top of
-- listings and gets a "wyróżnione" badge on the card. Not exposed
-- through the public event-create payload — only togglable via
-- POST /api/v1/admin/events/{id}/feature.

ALTER TABLE public.events
    ADD COLUMN is_featured BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX events_is_featured_idx ON public.events (is_featured) WHERE is_featured;

-- SECURITY DEFINER helper so the admin endpoint can toggle is_featured
-- without a viewer context (events is RLS-gated by viewer policies).
-- Returns true iff a row was updated. UPDATE-only — never creates events.
CREATE OR REPLACE FUNCTION app.admin_set_event_featured(p_event_id uuid, p_is_featured boolean)
RETURNS boolean
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    rows_updated integer;
BEGIN
    UPDATE public.events
    SET is_featured = p_is_featured,
        updated_at = now()
    WHERE id = p_event_id;
    GET DIAGNOSTICS rows_updated = ROW_COUNT;
    RETURN rows_updated > 0;
END;
$$;

REVOKE EXECUTE ON FUNCTION app.admin_set_event_featured(uuid, boolean) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.admin_set_event_featured(uuid, boolean) TO poziomki_api';
    END IF;
END
$$;
