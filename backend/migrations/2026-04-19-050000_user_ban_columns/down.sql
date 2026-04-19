DROP FUNCTION IF EXISTS app.admin_ban_user(uuid, text);
DROP INDEX IF EXISTS idx_users_banned_at;
ALTER TABLE public.users
    DROP COLUMN IF EXISTS banned_reason,
    DROP COLUMN IF EXISTS banned_at;
