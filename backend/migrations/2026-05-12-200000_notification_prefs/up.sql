-- Per-channel notification preferences + per-conversation mute.
--
-- Today `user_settings.notifications_enabled` is the only switch, and it
-- isn't surfaced anywhere in the UI. This migration adds the granular
-- channels the new Powiadomienia screen needs (DMs vs event chats vs
-- tag-driven event recommendations), and a `conversation_mutes` table
-- so users can silence individual chats from the chat header.
--
-- All new columns default TRUE so existing users keep getting pushes;
-- the master `notifications_enabled` switch stays the kill-all.

ALTER TABLE public.user_settings
    ADD COLUMN notify_dms boolean NOT NULL DEFAULT true,
    ADD COLUMN notify_event_chats boolean NOT NULL DEFAULT true,
    ADD COLUMN notify_tag_events boolean NOT NULL DEFAULT true;

CREATE TABLE public.conversation_mutes (
    user_id integer NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    conversation_id uuid NOT NULL REFERENCES public.conversations(id) ON DELETE CASCADE,
    muted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, conversation_id)
);

CREATE INDEX conversation_mutes_user_id_idx ON public.conversation_mutes(user_id);

ALTER TABLE public.conversation_mutes ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.conversation_mutes FORCE ROW LEVEL SECURITY;
CREATE POLICY conversation_mutes_viewer ON public.conversation_mutes
    FOR ALL TO poziomki_api
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());

-- Narrow SECURITY DEFINER helper: gate a list of recipient user_ids
-- against their notification preferences for a specific conversation.
-- Returns only the user_ids whose master switch is on, channel for the
-- conversation kind is on, and who haven't muted this conversation.
-- Server-side push delivery only; the API role doesn't need broad SELECT
-- across user_settings or conversation_mutes.
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

COMMENT ON FUNCTION app.push_targets_filtered(integer[], uuid) IS
    'Filter push recipients by per-channel preferences and per-conversation mute. Narrow projection; server-side push only.';

REVOKE EXECUTE ON FUNCTION app.push_targets_filtered(integer[], uuid) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.push_targets_filtered(integer[], uuid) TO poziomki_api';
    END IF;
END
$$;
