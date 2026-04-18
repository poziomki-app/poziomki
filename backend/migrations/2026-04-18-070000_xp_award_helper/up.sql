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
-- Date handling: explicit `(now() AT TIME ZONE 'UTC')::date` instead of
-- `CURRENT_DATE`, to match the pre-PR Rust behaviour
-- (`Utc::now().date_naive()`). `CURRENT_DATE` would depend on the Postgres
-- session timezone, so around UTC midnight streak preservation /
-- increment / reset could diverge from what the app expects.
--
-- Amount bound: `p_amount` is restricted to `(0, 100]`. Current callers
-- award 5 or 10; the bound keeps this helper from being repurposed to
-- subtract XP or inflate it arbitrarily if a future bug or SQL-injection
-- path reaches it. Out-of-range inputs raise and the UPDATE is skipped.
CREATE OR REPLACE FUNCTION app.award_profile_xp(p_profile_id uuid, p_amount int)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    today date := (now() AT TIME ZONE 'UTC')::date;
BEGIN
    IF p_amount IS NULL OR p_amount <= 0 OR p_amount > 100 THEN
        RAISE EXCEPTION 'award_profile_xp: p_amount out of range (got %, expected 1..100)', p_amount;
    END IF;

    UPDATE public.profiles
    SET xp = xp + p_amount,
        streak_current = CASE
            WHEN streak_last_active = today THEN streak_current
            WHEN streak_last_active = today - INTERVAL '1 day' THEN streak_current + 1
            ELSE 1
        END,
        streak_longest = GREATEST(streak_longest, CASE
            WHEN streak_last_active = today THEN streak_current
            WHEN streak_last_active = today - INTERVAL '1 day' THEN streak_current + 1
            ELSE 1
        END),
        streak_last_active = today
    WHERE id = p_profile_id;
END
$$;

COMMENT ON FUNCTION app.award_profile_xp(uuid, int) IS
    'Award XP + bump streak for a profile. Owner-level UPDATE so the scan_token handler and spawned award tasks can credit both parties without relying on viewer context. Amount bounded to 1..100; date evaluated in UTC.';

REVOKE EXECUTE ON FUNCTION app.award_profile_xp(uuid, int) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.award_profile_xp(uuid, int) TO poziomki_api';
    END IF;
END
$$;
