-- SECURITY DEFINER helpers used by the authentication path.
--
-- Login and session resolution must look up users/sessions BEFORE a viewer
-- context has been established, so once RLS is enabled on those tables
-- (Tier A) the normal SELECTs will return zero rows. These functions run as
-- the owner (BYPASSRLS) and are locked to exact-match inputs, so the API
-- role can call them to authenticate without being granted broad read on the
-- underlying tables.

CREATE SCHEMA IF NOT EXISTS app;

COMMENT ON SCHEMA app IS
    'Internal helpers invoked from the API layer (viewer context, auth lookups).';

-- Exact-match user lookup by email. Used by the login/signup flow to load a
-- row plus password hash for verification. Never expose fuzzy matching here.
CREATE OR REPLACE FUNCTION app.find_user_for_login(p_email text)
RETURNS TABLE (
    id int,
    pid uuid,
    name varchar,
    email varchar,
    password varchar,
    email_verified_at timestamptz,
    is_review_stub bool
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT u.id, u.pid, u.name, u.email, u.password, u.email_verified_at, u.is_review_stub
    FROM users u
    WHERE u.email = p_email
    LIMIT 1
$$;

COMMENT ON FUNCTION app.find_user_for_login(text) IS
    'Exact-match user lookup for the login path. Runs as owner (bypasses RLS) because it is called before a viewer context is established.';

-- Exact-match session resolution by hashed bearer token.
CREATE OR REPLACE FUNCTION app.resolve_session(p_token_hash text)
RETURNS TABLE (
    session_id uuid,
    user_id int,
    user_pid uuid,
    token varchar,
    ip_address varchar,
    user_agent varchar,
    expires_at timestamptz,
    created_at timestamptz,
    updated_at timestamptz,
    is_review_stub bool
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = public, pg_temp
STABLE
AS $$
    SELECT s.id, s.user_id, u.pid, s.token, s.ip_address, s.user_agent,
           s.expires_at, s.created_at, s.updated_at, u.is_review_stub
    FROM sessions s
    JOIN users u ON u.id = s.user_id
    WHERE s.token = p_token_hash
    LIMIT 1
$$;

COMMENT ON FUNCTION app.resolve_session(text) IS
    'Exact-match session resolution by hashed token. Runs as owner so the bearer-token middleware can authenticate before any viewer context exists.';

REVOKE EXECUTE ON FUNCTION app.find_user_for_login(text) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.resolve_session(text) FROM PUBLIC;

-- Grant to the API role if it exists. The role is created by
-- infra/ops/postgres/setup-roles.sh; pristine dev clones that have not run
-- that step will skip these grants and the owner retains default EXECUTE.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.find_user_for_login(text) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.resolve_session(text) TO poziomki_api';
    END IF;
END
$$;
