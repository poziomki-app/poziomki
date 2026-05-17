ALTER TABLE public.user_settings
    ALTER COLUMN notify_event_chats SET DEFAULT true,
    ALTER COLUMN notify_tag_events SET DEFAULT true;
