-- New users should default to DMs only. Event-chat and tag-event pushes
-- are opt-in (the user can flip them on in Powiadomienia). Existing rows
-- are intentionally left alone — anyone who already onboarded keeps the
-- channels they were granted.

ALTER TABLE public.user_settings
    ALTER COLUMN notify_event_chats SET DEFAULT false,
    ALTER COLUMN notify_tag_events SET DEFAULT false;
