-- Profile blocks (user-level blocking for DM conversations)
CREATE TABLE profile_blocks (
    blocker_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (blocker_id, blocked_id)
);

CREATE INDEX idx_profile_blocks_blocked ON profile_blocks(blocked_id);

-- Archive support on conversation_members
ALTER TABLE conversation_members ADD COLUMN archived_at TIMESTAMPTZ;
