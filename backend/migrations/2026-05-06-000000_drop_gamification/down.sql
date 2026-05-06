ALTER TABLE profiles
    ADD COLUMN IF NOT EXISTS xp INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS streak_current INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS streak_longest INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS streak_last_active DATE;

CREATE TABLE IF NOT EXISTS xp_scans (
    scanner_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    scanned_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    day DATE NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (scanner_id, scanned_id, day)
);

CREATE TABLE IF NOT EXISTS task_completions (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    task_id    TEXT NOT NULL,
    day        DATE NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (profile_id, task_id, day)
);

ALTER TABLE public.xp_scans ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.xp_scans FORCE ROW LEVEL SECURITY;
CREATE POLICY xp_scans_viewer ON public.xp_scans
    FOR ALL TO poziomki_api
    USING (scanner_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (scanner_id IN (SELECT id FROM app.viewer_profile_ids()));

ALTER TABLE public.task_completions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.task_completions FORCE ROW LEVEL SECURITY;
CREATE POLICY task_completions_viewer ON public.task_completions
    FOR ALL TO poziomki_api
    USING (profile_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (profile_id IN (SELECT id FROM app.viewer_profile_ids()));
