-- is_system marks accounts that exist only to author official content
-- (e.g. the "Poziomki" account that owns events curated from external
-- sources). System users must never appear in matching candidates,
-- people search, or tag-event push fanout. Their content (events,
-- messages they may post in event chats) stays visible — only the
-- author surface is hidden.

ALTER TABLE public.users
    ADD COLUMN is_system BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX users_is_system_idx ON public.users (is_system) WHERE is_system;

-- Refresh the tag-match push helper to also skip system authors. They
-- have no tags today, so this is belt-and-braces, but it keeps the
-- exclusion explicit alongside banned + stub.
CREATE OR REPLACE FUNCTION app.users_for_event_tag_match(
    p_event_id uuid,
    p_creator_user_id integer
)
RETURNS TABLE (user_id integer)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT DISTINCT p.user_id
    FROM public.profile_tags pt
    JOIN public.event_tags et ON et.tag_id = pt.tag_id
    JOIN public.profiles p ON p.id = pt.profile_id
    JOIN public.users u ON u.id = p.user_id
    LEFT JOIN public.user_settings s ON s.user_id = p.user_id
    WHERE et.event_id = p_event_id
      AND p.user_id <> p_creator_user_id
      AND u.banned_at IS NULL
      AND COALESCE(u.is_review_stub, FALSE) = FALSE
      AND COALESCE(u.is_system, FALSE) = FALSE
      AND COALESCE(s.notifications_enabled, TRUE)
      AND COALESCE(s.notify_tag_events, FALSE)
$$;

-- SECURITY DEFINER lookup of system user ids. The users RLS policy is
-- own-row, so a regular SELECT from the API role can't enumerate other
-- users. Matching needs the full set to exclude system authors from
-- candidates; narrow projection (just ids) so no sensitive columns
-- leak even if the caller is unexpected.
CREATE OR REPLACE FUNCTION app.system_user_ids()
RETURNS TABLE (id integer)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT id FROM public.users WHERE is_system = TRUE
$$;

REVOKE EXECUTE ON FUNCTION app.system_user_ids() FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.system_user_ids() TO poziomki_api';
    END IF;
END
$$;

-- Backfill the existing Poziomki account (created by hand on prod for
-- external event curation) so it stops appearing in discover.
UPDATE public.users SET is_system = TRUE WHERE email = 'system@poziomki.app';
