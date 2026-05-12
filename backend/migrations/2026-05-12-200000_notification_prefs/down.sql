DROP FUNCTION IF EXISTS app.push_targets_filtered(integer[], uuid);

DROP TABLE IF EXISTS public.conversation_mutes;

ALTER TABLE public.user_settings
    DROP COLUMN IF EXISTS notify_tag_events,
    DROP COLUMN IF EXISTS notify_event_chats,
    DROP COLUMN IF EXISTS notify_dms;
