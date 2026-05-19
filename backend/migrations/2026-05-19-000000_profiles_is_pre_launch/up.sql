-- Pre-launch (early-access) profiles are created via the landing page
-- before the user has installed the mobile app. We hide them from every
-- app-facing read (recommendations, search, /profiles/{id}, …) until
-- the owner opens the mobile app and finishes onboarding there — at
-- that point the mobile client calls
-- POST /api/v1/profiles/me/finalize-pre-launch which flips this flag
-- to FALSE.
--
-- Defaulting to FALSE means every pre-existing profile remains visible;
-- only profiles created by the landing branch get is_pre_launch = TRUE.
ALTER TABLE public.profiles
    ADD COLUMN IF NOT EXISTS is_pre_launch BOOLEAN NOT NULL DEFAULT FALSE;

-- Partial index keeps the hot path (visible profiles) cheap. Most
-- profiles will have is_pre_launch = FALSE forever, so a full index
-- would be wasted bytes.
CREATE INDEX IF NOT EXISTS idx_profiles_pre_launch
    ON public.profiles (is_pre_launch)
    WHERE is_pre_launch = TRUE;

-- SECURITY DEFINER proc the mobile-onboarding finalize endpoint calls.
-- Restricts the flip to the caller's own profile via the viewer
-- context so a malicious client can't reveal someone else's row.
CREATE OR REPLACE FUNCTION app.finalize_pre_launch_profile(p_user_id INT)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    updated INT;
BEGIN
    UPDATE public.profiles
        SET is_pre_launch = FALSE,
            updated_at = NOW()
        WHERE user_id = p_user_id
          AND is_pre_launch = TRUE;
    GET DIAGNOSTICS updated = ROW_COUNT;
    RETURN updated > 0;
END;
$$;

REVOKE ALL ON FUNCTION app.finalize_pre_launch_profile(INT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION app.finalize_pre_launch_profile(INT) TO poziomki_api;
