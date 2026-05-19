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

DROP FUNCTION IF EXISTS app.system_user_ids();
DROP INDEX IF EXISTS users_is_system_idx;
ALTER TABLE public.users DROP COLUMN is_system;
