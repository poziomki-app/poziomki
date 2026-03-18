CREATE TABLE recommendation_feedback (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    event_id   UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    feedback   VARCHAR NOT NULL CHECK (feedback IN ('more', 'less')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, event_id)
);

CREATE INDEX idx_recommendation_feedback_profile
    ON recommendation_feedback(profile_id);
