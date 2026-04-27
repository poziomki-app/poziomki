DROP TABLE IF EXISTS public.chat_message_reveals;

ALTER TABLE public.messages
    DROP COLUMN IF EXISTS moderation_verdict,
    DROP COLUMN IF EXISTS moderation_categories,
    DROP COLUMN IF EXISTS moderation_scanned_at;
