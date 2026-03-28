CREATE TABLE profile_bookmarks (
    profile_id       UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    target_profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, target_profile_id),
    CHECK (profile_id <> target_profile_id)
);

CREATE INDEX idx_profile_bookmarks_profile ON profile_bookmarks(profile_id);
