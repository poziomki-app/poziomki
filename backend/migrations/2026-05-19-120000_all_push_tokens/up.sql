-- SECURITY DEFINER helper for admin broadcast push.
-- Returns every registered FCM token regardless of user. Used only by
-- the admin broadcast endpoint, which does not honour per-user
-- notification preferences (broadcasts are operational announcements,
-- not chat). The narrow projection matches push_tokens_for_users —
-- never expose user_id to the caller.

CREATE OR REPLACE FUNCTION app.all_push_tokens()
RETURNS TABLE (fcm_token text, platform varchar)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT fcm_token, platform
    FROM public.push_subscriptions
$$;

COMMENT ON FUNCTION app.all_push_tokens() IS
    'Return every registered FCM token + platform. Admin broadcast path only; bypasses per-user notification preferences.';

REVOKE EXECUTE ON FUNCTION app.all_push_tokens() FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.all_push_tokens() TO poziomki_api';
    END IF;
END
$$;
