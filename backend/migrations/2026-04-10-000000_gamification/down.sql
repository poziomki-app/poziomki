DROP TABLE IF EXISTS xp_scans;

ALTER TABLE profiles
    DROP COLUMN IF EXISTS streak_last_active,
    DROP COLUMN IF EXISTS streak_longest,
    DROP COLUMN IF EXISTS streak_current,
    DROP COLUMN IF EXISTS xp;
