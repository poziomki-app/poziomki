ALTER TABLE profiles
    ADD COLUMN xp INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN streak_current INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN streak_longest INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN streak_last_active DATE;

CREATE TABLE xp_scans (
    scanner_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    scanned_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    day DATE NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (scanner_id, scanned_id, day)
);
