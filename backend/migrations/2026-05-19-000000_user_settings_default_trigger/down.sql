-- Restore the original INNER-JOIN filter.
CREATE OR REPLACE FUNCTION app.push_targets_filtered(
    p_user_ids integer[],
    p_conversation_id uuid
)
RETURNS TABLE (user_id integer)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT u.uid
    FROM unnest(p_user_ids) AS u(uid)
    JOIN public.user_settings s ON s.user_id = u.uid
    JOIN public.conversations c ON c.id = p_conversation_id
    WHERE s.notifications_enabled
      AND CASE c.kind
            WHEN 'dm'    THEN s.notify_dms
            WHEN 'event' THEN s.notify_event_chats
            ELSE TRUE
          END
      AND NOT EXISTS (
        SELECT 1 FROM public.conversation_mutes m
        WHERE m.user_id = u.uid
          AND m.conversation_id = p_conversation_id
      )
$$;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.push_targets_filtered(integer[], uuid) TO poziomki_api';
    END IF;
END
$$;

DROP TRIGGER IF EXISTS user_settings_default_after_user_insert ON public.users;
DROP FUNCTION IF EXISTS app.user_settings_default_for_new_user();

-- Backfilled rows are not reversed; deleting auto-created rows would
-- destroy user preferences and is not safe to undo blindly.
