-- Narrow public-projection helpers for cross-user profile reads.
--
-- A viewer rendering someone else's profile needs two non-sensitive facts
-- from the owner: their external id (users.pid) and whether the owner allows
-- their program to be shown (user_settings.privacy_show_program). Exposing
-- the full users / user_settings rows to the API role would be necessary
-- for the current handlers, but those tables carry sensitive columns
-- (password hash, email, private settings) that the viewer must not see
-- under a sane RLS policy.
--
-- The functions below run as owner and each returns a single value scoped
-- to exactly the column the caller needs, so Tier-A policies on users and
-- user_settings can be "own row only" without breaking public profile
-- rendering.

-- Return the external pid for a user_id, or NULL if the user doesn't exist.
CREATE OR REPLACE FUNCTION app.user_pid_for_id(p_user_id int)
RETURNS uuid
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT pid FROM users WHERE id = p_user_id
$$;

COMMENT ON FUNCTION app.user_pid_for_id(int) IS
    'Return the public pid for a user. Narrow projection; safe to call across viewers.';

-- Return the privacy_show_program flag for a user_id. Defaults to true when
-- there is no user_settings row (matches the previous is_none_or fallback).
CREATE OR REPLACE FUNCTION app.profile_program_visibility(p_user_id int)
RETURNS bool
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT COALESCE(
        (SELECT privacy_show_program FROM user_settings WHERE user_id = p_user_id),
        true
    )
$$;

COMMENT ON FUNCTION app.profile_program_visibility(int) IS
    'Whether a user has opted to show their program on their public profile. Defaults to true when no row exists.';

REVOKE EXECUTE ON FUNCTION app.user_pid_for_id(int) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.profile_program_visibility(int) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.user_pid_for_id(int) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.profile_program_visibility(int) TO poziomki_api';
    END IF;
END
$$;
