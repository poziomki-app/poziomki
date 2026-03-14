-- Chat tables: conversations, members, messages, reactions, push subscriptions.
-- Replaces Matrix (Tuwunel) with WebSocket + Postgres.

CREATE TABLE conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind VARCHAR(16) NOT NULL CHECK (kind IN ('dm', 'event')),
    title VARCHAR(255),
    event_id UUID REFERENCES events(id) ON DELETE SET NULL,
    -- For DMs: canonical user pair (lower id, higher id)
    user_low_id INT4 REFERENCES users(id),
    user_high_id INT4 REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One DM conversation per user pair
CREATE UNIQUE INDEX idx_conversations_dm_pair
    ON conversations (user_low_id, user_high_id)
    WHERE kind = 'dm';

-- One conversation per event
CREATE UNIQUE INDEX idx_conversations_event
    ON conversations (event_id)
    WHERE kind = 'event';

CREATE TABLE conversation_members (
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_read_message_id UUID,
    PRIMARY KEY (conversation_id, user_id)
);

CREATE INDEX idx_conversation_members_user ON conversation_members (user_id);

CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    kind VARCHAR(16) NOT NULL DEFAULT 'text' CHECK (kind IN ('text', 'image', 'file')),
    attachment_upload_id UUID REFERENCES uploads(id),
    reply_to_id UUID REFERENCES messages(id) ON DELETE SET NULL,
    -- Client-generated ID for idempotency
    client_id VARCHAR(64),
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_messages_conversation_created ON messages (conversation_id, created_at);
CREATE INDEX idx_messages_sender ON messages (sender_id);

CREATE TABLE message_reactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji VARCHAR(32) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (message_id, user_id, emoji)
);

CREATE INDEX idx_message_reactions_message ON message_reactions (message_id);

CREATE TABLE push_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id VARCHAR(64) NOT NULL,
    ntfy_topic VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, device_id)
);

-- FK for read receipt watermark (deferred to avoid circular dependency at INSERT time)
ALTER TABLE conversation_members
    ADD CONSTRAINT fk_last_read_message
    FOREIGN KEY (last_read_message_id) REFERENCES messages(id) ON DELETE SET NULL;

-- Autovacuum tuning for messages table (high write volume)
ALTER TABLE messages SET (
    autovacuum_vacuum_threshold = 100,
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_threshold = 100
);
