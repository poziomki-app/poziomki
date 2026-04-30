-- Add platform + apns_token columns so iOS clients can register APNs device
-- tokens alongside Android ntfy topics. Existing rows are Android.
ALTER TABLE push_subscriptions
    ADD COLUMN platform VARCHAR(16) NOT NULL DEFAULT 'android',
    ADD COLUMN apns_token VARCHAR(200);

ALTER TABLE push_subscriptions
    ALTER COLUMN ntfy_topic DROP NOT NULL;

-- Exactly one of (ntfy_topic, apns_token) must be set, matching platform.
ALTER TABLE push_subscriptions
    ADD CONSTRAINT push_subscriptions_platform_token CHECK (
        (platform = 'android' AND ntfy_topic IS NOT NULL AND apns_token IS NULL) OR
        (platform = 'ios' AND apns_token IS NOT NULL AND ntfy_topic IS NULL)
    );

-- Drop and recreate the SECURITY DEFINER helper to expose platform + token
-- columns to notify_push so it can dispatch to the right provider.
DROP FUNCTION IF EXISTS app.push_topics_for_users(int[]);

CREATE OR REPLACE FUNCTION app.push_subscriptions_for_users(p_user_ids int[])
RETURNS TABLE (platform varchar, ntfy_topic varchar, apns_token varchar)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT platform, ntfy_topic, apns_token
    FROM public.push_subscriptions
    WHERE user_id = ANY(p_user_ids)
$$;

COMMENT ON FUNCTION app.push_subscriptions_for_users(int[]) IS
    'Return platform + push tokens (ntfy or apns) for a set of users. Narrow projection; server-side push delivery only.';

REVOKE EXECUTE ON FUNCTION app.push_subscriptions_for_users(int[]) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.push_subscriptions_for_users(int[]) TO poziomki_api';
    END IF;
END
$$;
