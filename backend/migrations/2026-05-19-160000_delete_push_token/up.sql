-- SECURITY DEFINER helper so the API role can prune dead FCM tokens.
--
-- `push_subscriptions` is RLS-protected by user_id, and the API role
-- runs without a viewer context for server-side push delivery. That
-- meant `cleanup_stale_token` in src/push/fcm.rs silently no-op'd
-- (DELETE matched zero rows under RLS) and dead tokens accumulated
-- forever — every broadcast wasted FCM quota on them. This helper
-- bypasses RLS for the narrow "delete by exact token value" case
-- without exposing user identities to the caller.

CREATE OR REPLACE FUNCTION app.delete_push_token(p_fcm_token text)
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    DELETE FROM public.push_subscriptions
    WHERE fcm_token = p_fcm_token
$$;

COMMENT ON FUNCTION app.delete_push_token(text) IS
    'Delete a stale FCM token. Called from the server-side push path after FCM rejects a token (UNREGISTERED / INVALID_ARGUMENT / 404). Bypasses RLS to prune dead rows.';

REVOKE EXECUTE ON FUNCTION app.delete_push_token(text) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.delete_push_token(text) TO poziomki_api';
    END IF;
END
$$;
