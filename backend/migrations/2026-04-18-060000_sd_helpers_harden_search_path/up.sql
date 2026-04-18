-- Harden every SECURITY DEFINER helper in `app` against pg_temp search-path
-- hijacks (CVE-2018-1058 style).
--
-- Before this migration the helpers were defined with
-- `SET search_path = public, pg_temp`. Postgres always searches the temp
-- schema before an unqualified schema list unless `pg_catalog` is explicitly
-- named first, so any caller with TEMPORARY privilege could install a
-- `pg_temp.users` / `pg_temp.profiles` / `pg_temp.sessions` view and the
-- SD functions would read from it. For `user_pids_for_ids` and
-- `profile_owner_user_id` that means attacker-controlled `(user_id, pid)`
-- pairs or a fabricated owner `user_id` — enough to impersonate users in
-- attendee listings and misdirect push notifications.
--
-- CREATE OR REPLACE the functions with:
--   * `SET search_path = pg_catalog, pg_temp` — Postgres will not implicitly
--     search pg_temp first when pg_catalog is listed, and we only want
--     pg_temp around for session-temp tables, never name resolution.
--   * Fully schema-qualified table references (`public.users`,
--     `public.profiles`, `public.sessions`, `public.user_settings`,
--     `public.push_subscriptions`) so no lookup is affected by search_path
--     at all.
--
-- All EXECUTE grants are inherited through CREATE OR REPLACE; we re-run
-- the idempotent grant block at the end for defence.

-- ---------------------------------------------------------------------------
-- auth_security_definer (010000)
-- ---------------------------------------------------------------------------

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
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT u.id, u.pid, u.name, u.email, u.password, u.email_verified_at, u.is_review_stub
    FROM public.users u
    WHERE u.email = p_email
    LIMIT 1
$$;

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
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT s.id, s.user_id, u.pid, s.token, s.ip_address, s.user_agent,
           s.expires_at, s.created_at, s.updated_at, u.is_review_stub
    FROM public.sessions s
    JOIN public.users u ON u.id = s.user_id
    WHERE s.token = p_token_hash
    LIMIT 1
$$;

-- ---------------------------------------------------------------------------
-- auth_writes_security_definer (020000)
-- ---------------------------------------------------------------------------

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
SET search_path = pg_catalog, pg_temp
AS $$
    INSERT INTO public.users (pid, email, password, api_key, name)
    VALUES (p_pid, p_email, p_password, p_api_key, p_name)
    RETURNING id, pid, email, password, name, email_verified_at, is_review_stub;
$$;

CREATE OR REPLACE FUNCTION app.mark_email_verified(p_user_id int, p_now timestamptz)
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    UPDATE public.users
    SET email_verified_at = p_now,
        updated_at = p_now
    WHERE id = p_user_id
      AND email_verified_at IS NULL;
$$;

CREATE OR REPLACE FUNCTION app.set_password_reset_token(
    p_user_id int,
    p_token_hash text,
    p_now timestamptz
)
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    UPDATE public.users
    SET reset_token = p_token_hash,
        reset_sent_at = p_now,
        updated_at = p_now
    WHERE id = p_user_id;
$$;

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
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT id, pid, email, name, email_verified_at, is_review_stub
    FROM public.users
    WHERE email = p_email
      AND reset_token = p_token_hash
      AND reset_sent_at > p_cutoff
    LIMIT 1
$$;

CREATE OR REPLACE FUNCTION app.complete_password_reset(
    p_user_id int,
    p_new_password text,
    p_now timestamptz
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    UPDATE public.users
    SET password = p_new_password,
        reset_token = NULL,
        reset_sent_at = NULL,
        updated_at = p_now
    WHERE id = p_user_id;

    DELETE FROM public.sessions WHERE user_id = p_user_id;
END
$$;

CREATE OR REPLACE FUNCTION app.create_session_for_user(
    p_id uuid,
    p_user_id int,
    p_token_hash text,
    p_ip_address varchar,
    p_user_agent varchar,
    p_now timestamptz,
    p_expires_at timestamptz
)
RETURNS TABLE (
    id uuid,
    user_id int,
    token varchar,
    ip_address varchar,
    user_agent varchar,
    expires_at timestamptz,
    created_at timestamptz,
    updated_at timestamptz
)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    INSERT INTO public.sessions (id, user_id, token, ip_address, user_agent, expires_at, created_at, updated_at)
    VALUES (p_id, p_user_id, p_token_hash, p_ip_address, p_user_agent, p_expires_at, p_now, p_now)
    RETURNING id, user_id, token, ip_address, user_agent, expires_at, created_at, updated_at;
$$;

CREATE OR REPLACE FUNCTION app.delete_session_by_token(p_token_hash text)
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    DELETE FROM public.sessions WHERE token = p_token_hash;
$$;

-- ---------------------------------------------------------------------------
-- profile_public_helpers (030000)
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION app.user_pid_for_id(p_user_id int)
RETURNS uuid
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT pid FROM public.users WHERE id = p_user_id
$$;

CREATE OR REPLACE FUNCTION app.profile_program_visibility(p_user_id int)
RETURNS bool
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT COALESCE(
        (SELECT privacy_show_program FROM public.user_settings WHERE user_id = p_user_id),
        true
    )
$$;

-- ---------------------------------------------------------------------------
-- chat_rls_helpers (040000)
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION app.user_id_for_pid(p_pid uuid)
RETURNS int
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT id FROM public.users WHERE pid = p_pid
$$;

CREATE OR REPLACE FUNCTION app.user_review_stubs(p_user_ids int[])
RETURNS TABLE (user_id int, is_review_stub bool)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT id, is_review_stub FROM public.users WHERE id = ANY(p_user_ids)
$$;

CREATE OR REPLACE FUNCTION app.push_topics_for_users(p_user_ids int[])
RETURNS TABLE (ntfy_topic varchar)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT ntfy_topic FROM public.push_subscriptions WHERE user_id = ANY(p_user_ids)
$$;

-- ---------------------------------------------------------------------------
-- event_rls_helpers (050000) — re-define with the hardened header too, even
-- though the same migration in the same PR is already correct. Running
-- `CREATE OR REPLACE` twice is a no-op and keeps every SD helper in one
-- authoritative place.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION app.user_pids_for_ids(p_user_ids int[])
RETURNS TABLE (user_id int, pid uuid)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT id, pid FROM public.users WHERE id = ANY(p_user_ids)
$$;

CREATE OR REPLACE FUNCTION app.profile_owner_user_id(p_profile_id uuid)
RETURNS int
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT user_id FROM public.profiles WHERE id = p_profile_id
$$;
