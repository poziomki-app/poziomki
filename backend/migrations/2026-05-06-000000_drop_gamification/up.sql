DROP FUNCTION IF EXISTS app.award_profile_xp(uuid, int);
DROP TABLE IF EXISTS task_completions CASCADE;
DROP TABLE IF EXISTS xp_scans CASCADE;

ALTER TABLE profiles
    DROP COLUMN IF EXISTS xp,
    DROP COLUMN IF EXISTS streak_current,
    DROP COLUMN IF EXISTS streak_longest,
    DROP COLUMN IF EXISTS streak_last_active;
