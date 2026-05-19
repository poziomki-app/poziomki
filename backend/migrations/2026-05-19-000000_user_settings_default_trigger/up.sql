-- Guarantee every user has a `user_settings` row.
--
-- Push delivery filters recipients through `app.push_targets_filtered`,
-- which historically INNER-JOIN'd `user_settings`. The settings row was
-- only inserted lazily by `PUT /settings` (api/settings.rs) — users who
-- never opened the Powiadomienia screen had no row, and the inner join
-- silently dropped them, so they never received any push. Prod was
-- carrying 16/36 users in that broken state until we noticed Celina
-- wasn't getting Dawid's DMs.
--
-- Three-layer fix:
--   1. backfill any user missing a settings row, using schema defaults
--   2. AFTER INSERT trigger on `public.users` so every future user gets
--      a row automatically, regardless of which code path inserts them
--   3. relax `app.push_targets_filtered` to LEFT JOIN + COALESCE so push
--      keeps working with sensible defaults even if (1)/(2) ever miss

-- 1. Backfill -----------------------------------------------------------
-- Several user_settings columns are NOT NULL with no schema default
-- (theme/language/privacy_*/notifications_enabled), so we have to spell
-- the defaults out here. These match the values the lazy PUT /settings
-- handler creates (api/settings.rs).
INSERT INTO public.user_settings (
    user_id, theme, language,
    notifications_enabled, privacy_show_age, privacy_show_program, privacy_discoverable
)
SELECT u.id, 'system', 'pl', TRUE, TRUE, TRUE, TRUE
FROM public.users u
WHERE NOT EXISTS (
    SELECT 1 FROM public.user_settings s WHERE s.user_id = u.id
);

-- 2. Trigger ------------------------------------------------------------
CREATE OR REPLACE FUNCTION app.user_settings_default_for_new_user()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    INSERT INTO public.user_settings (
        user_id, theme, language,
        notifications_enabled, privacy_show_age, privacy_show_program, privacy_discoverable
    )
    VALUES (NEW.id, 'system', 'pl', TRUE, TRUE, TRUE, TRUE)
    ON CONFLICT (user_id) DO NOTHING;
    RETURN NEW;
END;
$$;

REVOKE EXECUTE ON FUNCTION app.user_settings_default_for_new_user() FROM PUBLIC;

DROP TRIGGER IF EXISTS user_settings_default_after_user_insert ON public.users;
CREATE TRIGGER user_settings_default_after_user_insert
AFTER INSERT ON public.users
FOR EACH ROW EXECUTE FUNCTION app.user_settings_default_for_new_user();

-- 3. Resilient filter ---------------------------------------------------
-- Defense in depth: even if a settings row is somehow missing (manual
-- DB intervention, future schema migration, etc.) push still works
-- using the same defaults the schema declares for new columns.
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
    JOIN public.conversations c ON c.id = p_conversation_id
    LEFT JOIN public.user_settings s ON s.user_id = u.uid
    WHERE COALESCE(s.notifications_enabled, TRUE)
      AND CASE c.kind
            WHEN 'dm'    THEN COALESCE(s.notify_dms, TRUE)
            WHEN 'event' THEN COALESCE(s.notify_event_chats, FALSE)
            ELSE TRUE
          END
      AND NOT EXISTS (
        SELECT 1 FROM public.conversation_mutes m
        WHERE m.user_id = u.uid
          AND m.conversation_id = p_conversation_id
      )
$$;

COMMENT ON FUNCTION app.push_targets_filtered(integer[], uuid) IS
    'Filter push recipients by per-channel preferences and per-conversation mute. LEFT JOIN with COALESCE so missing user_settings rows fall back to schema defaults instead of silently dropping.';

REVOKE EXECUTE ON FUNCTION app.push_targets_filtered(integer[], uuid) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.push_targets_filtered(integer[], uuid) TO poziomki_api';
    END IF;
END
$$;
