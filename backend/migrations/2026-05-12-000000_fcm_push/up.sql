-- Switch push transport from ntfy to FCM.
--
-- The column carries an opaque device-bound credential either way, so we
-- just rename it (`ntfy_topic` → `fcm_token`) and widen the type — FCM
-- registration tokens are unbounded by spec (~163 chars today). We also
-- add a `platform` discriminator so the server can pick the correct
-- payload envelope (android vs apns) at send time, and a
-- `token_updated_at` for triaging stale registrations.

ALTER TABLE public.push_subscriptions
    ALTER COLUMN ntfy_topic TYPE text,
    ADD COLUMN platform varchar(16) NOT NULL DEFAULT 'android'
        CHECK (platform IN ('android', 'ios')),
    ADD COLUMN token_updated_at timestamptz NOT NULL DEFAULT now();

ALTER TABLE public.push_subscriptions
    RENAME COLUMN ntfy_topic TO fcm_token;

-- Replace the narrow SECURITY DEFINER helper. Same shape (server-side
-- delivery only, no device ids or user ids exposed), just returning the
-- FCM token + platform tuple instead of an ntfy topic.
DROP FUNCTION IF EXISTS app.push_topics_for_users(int[]);

CREATE OR REPLACE FUNCTION app.push_tokens_for_users(p_user_ids int[])
RETURNS TABLE (fcm_token text, platform varchar)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT fcm_token, platform
    FROM public.push_subscriptions
    WHERE user_id = ANY(p_user_ids)
$$;

COMMENT ON FUNCTION app.push_tokens_for_users(int[]) IS
    'Return FCM tokens + platform for a set of users. Narrow projection; server-side push delivery only.';

REVOKE EXECUTE ON FUNCTION app.push_tokens_for_users(int[]) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.push_tokens_for_users(int[]) TO poziomki_api';
    END IF;
END
$$;
