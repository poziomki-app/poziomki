DROP FUNCTION IF EXISTS app.push_tokens_for_users(int[]);

ALTER TABLE public.push_subscriptions
    RENAME COLUMN fcm_token TO ntfy_topic;

ALTER TABLE public.push_subscriptions
    DROP COLUMN IF EXISTS platform,
    DROP COLUMN IF EXISTS token_updated_at,
    ALTER COLUMN ntfy_topic TYPE varchar(128);

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
