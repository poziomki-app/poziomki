DROP FUNCTION IF EXISTS app.claim_welcome_email_send(integer);
DROP FUNCTION IF EXISTS app.mark_welcome_email_sent(integer, timestamptz);
DROP FUNCTION IF EXISTS app.set_pre_launch_signup_metadata(uuid, text, text);

DROP INDEX IF EXISTS idx_users_pre_launch_signed_up_at;

ALTER TABLE public.users
    DROP CONSTRAINT IF EXISTS users_platform_pref_chk;

ALTER TABLE public.users
    DROP COLUMN IF EXISTS welcome_email_sent_at,
    DROP COLUMN IF EXISTS signup_source,
    DROP COLUMN IF EXISTS platform_pref,
    DROP COLUMN IF EXISTS pre_launch_signed_up_at;
