-- SECURITY DEFINER helper for the XP / streak award path.
--
-- XP is awarded from several spawned background tasks (scan completion,
-- event attend, event approve) and from the scan_token handler which
-- credits *both* the scanner and the scanned profile. Under Tier-A
-- `profiles` RLS (`USING (user_id = app.current_user_id())`), neither
-- the spawned task's anon context nor the scanner's viewer context can
-- UPDATE the other user's row. Moving the award to an `app.*` SECURITY
-- DEFINER function keeps the mutation narrow (single row, fixed columns)
-- while letting it run as the owner.
--
-- Streak logic: the targeted row's streak is either preserved (same
-- day), incremented (consecutive day), or reset to 1 (any other gap).
-- Matches the inline SQL it replaces in `backend/src/api/xp/service.rs`.
CREATE OR REPLACE FUNCTION app.award_profile_xp(p_profile_id uuid, p_amount int)
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    UPDATE public.profiles
    SET xp = xp + p_amount,
        streak_current = CASE
            WHEN streak_last_active = CURRENT_DATE THEN streak_current
            WHEN streak_last_active = CURRENT_DATE - INTERVAL '1 day' THEN streak_current + 1
            ELSE 1
        END,
        streak_longest = GREATEST(streak_longest, CASE
            WHEN streak_last_active = CURRENT_DATE THEN streak_current
            WHEN streak_last_active = CURRENT_DATE - INTERVAL '1 day' THEN streak_current + 1
            ELSE 1
        END),
        streak_last_active = CURRENT_DATE
    WHERE id = p_profile_id
$$;

COMMENT ON FUNCTION app.award_profile_xp(uuid, int) IS
    'Award XP + bump streak for a profile. Owner-level UPDATE so the scan_token handler and spawned award tasks can credit both parties without relying on viewer context.';

REVOKE EXECUTE ON FUNCTION app.award_profile_xp(uuid, int) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.award_profile_xp(uuid, int) TO poziomki_api';
    END IF;
END
$$;
