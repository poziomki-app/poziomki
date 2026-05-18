-- Pre-launch (early-access) signup metadata on users.
--
-- Early adopters who register via the landing page form (`source =
-- "landing_early_access"`) become real users — we just stamp them so we can
-- (a) generate CSVs for Play Internal Testing / TestFlight rosters and
-- (b) treat them specially later (launch-day perks, communications).
--
-- All columns are NULL-able so existing rows stay valid without a backfill.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS pre_launch_signed_up_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS platform_pref VARCHAR(16),
    ADD COLUMN IF NOT EXISTS signup_source VARCHAR(32),
    ADD COLUMN IF NOT EXISTS welcome_email_sent_at TIMESTAMPTZ;

-- Allow only the values the API writes; cheap CHECK keeps bad data out at
-- the DB boundary even if a future handler forgets to validate.
ALTER TABLE public.users
    ADD CONSTRAINT users_platform_pref_chk
        CHECK (platform_pref IS NULL OR platform_pref IN ('android', 'ios', 'either'));

-- Partial index — covers only the pre-launch subset, which is what the
-- export CLI and any "early adopter" filters scan.
CREATE INDEX IF NOT EXISTS idx_users_pre_launch_signed_up_at
    ON public.users (pre_launch_signed_up_at)
    WHERE pre_launch_signed_up_at IS NOT NULL;

-- SECURITY DEFINER helper for the sign-up path. The pre-auth flow runs
-- without a viewer context (see comment on `app.create_user_for_signup`
-- in the consolidated migration); the API role doesn't have direct
-- UPDATE on the new columns, so we expose a narrow helper that the
-- signup handler calls right after user creation.
--
-- Idempotent: if `pre_launch_signed_up_at` is already set, we don't
-- overwrite it on retry — the original timestamp is the one we care
-- about for cohort analysis.
CREATE OR REPLACE FUNCTION app.set_pre_launch_signup_metadata(
    p_user_pid uuid,
    p_platform_pref text,
    p_signup_source text
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    UPDATE public.users
    SET pre_launch_signed_up_at = COALESCE(pre_launch_signed_up_at, now()),
        platform_pref = COALESCE(platform_pref, p_platform_pref),
        signup_source = COALESCE(signup_source, p_signup_source),
        updated_at = now()
    WHERE pid = p_user_pid;
END
$$;

COMMENT ON FUNCTION app.set_pre_launch_signup_metadata(uuid, text, text) IS
    'Stamp a user as a pre-launch signup. Idempotent on first-set columns. Called by /api/v1/auth/sign-up/email when source=landing_early_access.';

REVOKE EXECUTE ON FUNCTION app.set_pre_launch_signup_metadata(uuid, text, text) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.set_pre_launch_signup_metadata(uuid, text, text) TO poziomki_api';
    END IF;
END
$$;

-- Companion helper for the post-onboarding welcome email job. Worker
-- writes go through the worker role, which also lacks direct UPDATE
-- on users — same SECURITY DEFINER pattern.
CREATE OR REPLACE FUNCTION app.mark_welcome_email_sent(
    p_user_id integer,
    p_sent_at timestamptz
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    UPDATE public.users
    SET welcome_email_sent_at = COALESCE(welcome_email_sent_at, p_sent_at),
        updated_at = now()
    WHERE id = p_user_id;
END
$$;

REVOKE EXECUTE ON FUNCTION app.mark_welcome_email_sent(integer, timestamptz) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_worker') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.mark_welcome_email_sent(integer, timestamptz) TO poziomki_worker';
    END IF;
END
$$;

-- Atomic claim: returns (email, name) iff this user is a pre-launch
-- signup whose welcome email hasn't been sent yet, AND stamps the
-- timestamp in the same statement. The worker calls this before
-- delivering, so concurrent retries can never produce duplicate sends.
--
-- Returns NULL row when the user is non-pre-launch or already received
-- the welcome — the worker treats that as a no-op success.
CREATE OR REPLACE FUNCTION app.claim_welcome_email_send(p_user_id integer)
RETURNS TABLE(email text, name text)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    RETURN QUERY
    UPDATE public.users
    SET welcome_email_sent_at = now(),
        updated_at = now()
    WHERE id = p_user_id
      AND pre_launch_signed_up_at IS NOT NULL
      AND welcome_email_sent_at IS NULL
    RETURNING public.users.email::text, public.users.name::text;
END
$$;

REVOKE EXECUTE ON FUNCTION app.claim_welcome_email_send(integer) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_worker') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.claim_welcome_email_send(integer) TO poziomki_worker';
    END IF;
END
$$;
