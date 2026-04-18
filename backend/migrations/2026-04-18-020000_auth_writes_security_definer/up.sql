-- SECURITY DEFINER helpers for pre-authentication writes to `users`.
--
-- Sign-up, email verification, and password reset all mutate the users table
-- before a viewer context exists. Once Tier-A policies are enabled, anon
-- connections cannot INSERT/UPDATE `users` directly; these helpers run as the
-- owner (BYPASSRLS) and are tightly scoped so the API role can call them for
-- exactly those flows and nothing else.

-- Insert a new user row for the sign-up flow. Returns the inserted row.
CREATE OR REPLACE FUNCTION app.create_user_for_signup(
    p_pid uuid,
    p_email text,
    p_password text,
    p_api_key text,
    p_name text
)
RETURNS TABLE (
    id int,
    pid uuid,
    email varchar,
    password varchar,
    name varchar,
    email_verified_at timestamptz,
    is_review_stub bool
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    INSERT INTO users (pid, email, password, api_key, name)
    VALUES (p_pid, p_email, p_password, p_api_key, p_name)
    RETURNING id, pid, email, password, name, email_verified_at, is_review_stub;
$$;

COMMENT ON FUNCTION app.create_user_for_signup(uuid, text, text, text, text) IS
    'Insert a user row for the sign-up flow. Runs as owner so it works before any viewer context exists.';

-- Mark an email as verified (sets email_verified_at + bumps updated_at).
CREATE OR REPLACE FUNCTION app.mark_email_verified(p_user_id int, p_now timestamptz)
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    UPDATE users
    SET email_verified_at = p_now,
        updated_at = p_now
    WHERE id = p_user_id
      AND email_verified_at IS NULL;
$$;

COMMENT ON FUNCTION app.mark_email_verified(int, timestamptz) IS
    'Set email_verified_at on a user. Idempotent; no-ops if already verified.';

-- Store a password-reset token for a user. Caller passes the SHA-256 hash.
CREATE OR REPLACE FUNCTION app.set_password_reset_token(
    p_user_id int,
    p_token_hash text,
    p_now timestamptz
)
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
    UPDATE users
    SET reset_token = p_token_hash,
        reset_sent_at = p_now,
        updated_at = p_now
    WHERE id = p_user_id;
$$;

COMMENT ON FUNCTION app.set_password_reset_token(int, text, timestamptz) IS
    'Record a hashed reset token on a user. Caller must have already verified OTP.';

-- Multi-filter lookup for the reset-password confirm step: matches on
-- email + exact hashed token + not-expired cutoff.
CREATE OR REPLACE FUNCTION app.find_user_for_password_reset(
    p_email text,
    p_token_hash text,
    p_cutoff timestamptz
)
RETURNS TABLE (
    id int,
    pid uuid,
    email varchar,
    name varchar,
    email_verified_at timestamptz,
    is_review_stub bool
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT id, pid, email, name, email_verified_at, is_review_stub
    FROM users
    WHERE email = p_email
      AND reset_token = p_token_hash
      AND reset_sent_at > p_cutoff
    LIMIT 1
$$;

COMMENT ON FUNCTION app.find_user_for_password_reset(text, text, timestamptz) IS
    'Exact-match user lookup for the reset-password confirm step. Caller verifies TTL via p_cutoff.';

-- Apply a completed password reset: rotate the password hash, clear the
-- reset-token columns, and invalidate any existing sessions.
CREATE OR REPLACE FUNCTION app.complete_password_reset(
    p_user_id int,
    p_new_password text,
    p_now timestamptz
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
BEGIN
    UPDATE users
    SET password = p_new_password,
        reset_token = NULL,
        reset_sent_at = NULL,
        updated_at = p_now
    WHERE id = p_user_id;

    DELETE FROM sessions WHERE user_id = p_user_id;
END
$$;

COMMENT ON FUNCTION app.complete_password_reset(int, text, timestamptz) IS
    'Rotate password hash, clear reset token, and invalidate all sessions for a user.';

REVOKE EXECUTE ON FUNCTION app.create_user_for_signup(uuid, text, text, text, text) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.mark_email_verified(int, timestamptz) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.set_password_reset_token(int, text, timestamptz) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.find_user_for_password_reset(text, text, timestamptz) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.complete_password_reset(int, text, timestamptz) FROM PUBLIC;

-- Grants to poziomki_api are covered by the ALTER DEFAULT PRIVILEGES clause
-- applied in the earlier auth_security_definer migration; any new function in
-- the `app` schema is auto-granted EXECUTE. Nothing extra is needed here.
