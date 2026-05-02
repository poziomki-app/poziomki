CREATE TABLE IF NOT EXISTS recommendation_feedback (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    feedback VARCHAR NOT NULL CHECK (feedback IN ('more', 'less')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, event_id)
);

CREATE INDEX IF NOT EXISTS idx_recommendation_feedback_profile
    ON recommendation_feedback(profile_id);

ALTER TABLE public.recommendation_feedback ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.recommendation_feedback FORCE ROW LEVEL SECURITY;
CREATE POLICY recommendation_feedback_viewer ON public.recommendation_feedback
    FOR ALL TO poziomki_api
    USING (profile_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (profile_id IN (SELECT id FROM app.viewer_profile_ids()));
