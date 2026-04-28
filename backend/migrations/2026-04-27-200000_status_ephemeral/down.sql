DROP INDEX IF EXISTS idx_profiles_status_expires_at;

ALTER TABLE public.profiles
    DROP CONSTRAINT IF EXISTS profiles_status_emoji_length,
    DROP COLUMN IF EXISTS status_emoji,
    DROP COLUMN IF EXISTS status_expires_at;
