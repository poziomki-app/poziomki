-- SECURITY DEFINER helper for tag-matched event push notifications.
--
-- Returns user_ids whose profile_tags overlap with a given event's
-- event_tags, gated by user_settings (master switch on, notify_tag_events
-- on — resilient COALESCE defaults so missing rows fall back to schema
-- defaults), excluding the creator and banned/review-stub users.
--
-- Server-side push delivery only. Narrow projection: user_id only —
-- the API role can fetch tokens via push_tokens_for_users.

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
      AND COALESCE(s.notifications_enabled, TRUE)
      AND COALESCE(s.notify_tag_events, FALSE)
$$;

COMMENT ON FUNCTION app.users_for_event_tag_match(uuid, integer) IS
    'Resolve users who opted in to tag-event push and have at least one matching tag on this event. Server-side push only; bypasses RLS.';

REVOKE EXECUTE ON FUNCTION app.users_for_event_tag_match(uuid, integer) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.users_for_event_tag_match(uuid, integer) TO poziomki_api';
    END IF;
END
$$;
