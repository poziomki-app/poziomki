DROP FUNCTION IF EXISTS app.push_subscriptions_for_users(int[]);

CREATE OR REPLACE FUNCTION app.push_topics_for_users(p_user_ids int[])
RETURNS TABLE (ntfy_topic varchar)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT ntfy_topic FROM public.push_subscriptions WHERE user_id = ANY(p_user_ids)
$$;

REVOKE EXECUTE ON FUNCTION app.push_topics_for_users(int[]) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.push_topics_for_users(int[]) TO poziomki_api';
    END IF;
END
$$;

ALTER TABLE push_subscriptions DROP CONSTRAINT IF EXISTS push_subscriptions_platform_token;

-- Restore ntfy_topic NOT NULL. iOS rows have no ntfy topic and would
-- break notify_push (POST {ntfy_server}/<empty>) if backfilled with an
-- empty string, so drop them — the pre-migration schema cannot represent
-- iOS subscriptions anyway.
DELETE FROM push_subscriptions WHERE ntfy_topic IS NULL;
ALTER TABLE push_subscriptions ALTER COLUMN ntfy_topic SET NOT NULL;

ALTER TABLE push_subscriptions DROP COLUMN apns_token;
ALTER TABLE push_subscriptions DROP COLUMN platform;
