-- Narrow public-projection helpers used by the chat module.
--
-- Chat handlers need a few small facts from `users` and `push_subscriptions`
-- that would otherwise require cross-user SELECT on tables carrying
-- sensitive columns (password hash, reset token, etc.). These SD helpers
-- expose exactly what the caller needs and no more, so the API role can
-- stay at least-privilege and Tier-A RLS on users/push_subscriptions can
-- stay "own row only".

-- Resolve the internal int id for a user's external pid. Returns NULL if
-- the pid is not known.
CREATE OR REPLACE FUNCTION app.user_id_for_pid(p_pid uuid)
RETURNS int
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT id FROM users WHERE pid = p_pid
$$;

COMMENT ON FUNCTION app.user_id_for_pid(uuid) IS
    'Resolve a user pid to its int id. Narrow projection; used by DM target lookup.';

-- Batch lookup of is_review_stub for a list of user ids. Used to hide DMs
-- between stub and non-stub accounts.
CREATE OR REPLACE FUNCTION app.user_review_stubs(p_user_ids int[])
RETURNS TABLE (user_id int, is_review_stub bool)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT id, is_review_stub FROM users WHERE id = ANY(p_user_ids)
$$;

COMMENT ON FUNCTION app.user_review_stubs(int[]) IS
    'Batch lookup of is_review_stub for a list of users. Narrow projection; used by chat DM filtering.';

-- Batch lookup of ntfy push topics for a list of user ids. Used by
-- notify_push to dispatch wake-up signals. Only the topic string is
-- exposed — no device identifiers, creation timestamps, or user ids.
CREATE OR REPLACE FUNCTION app.push_topics_for_users(p_user_ids int[])
RETURNS TABLE (ntfy_topic varchar)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT ntfy_topic FROM push_subscriptions WHERE user_id = ANY(p_user_ids)
$$;

COMMENT ON FUNCTION app.push_topics_for_users(int[]) IS
    'Return ntfy topics for a set of users. Narrow projection; server-side push delivery only.';

REVOKE EXECUTE ON FUNCTION app.user_id_for_pid(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.user_review_stubs(int[]) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.push_topics_for_users(int[]) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.user_id_for_pid(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.user_review_stubs(int[]) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.push_topics_for_users(int[]) TO poziomki_api';
    END IF;
END
$$;
