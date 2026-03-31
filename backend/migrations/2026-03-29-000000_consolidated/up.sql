-- Consolidated schema for Poziomki backend
-- Squashed from 20 incremental migrations (2026-02-23 through 2026-03-22)

-- Extensions
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS cube;
CREATE EXTENSION IF NOT EXISTS earthdistance;

-- ============================================================
-- Core tables
-- ============================================================

CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    pid UUID NOT NULL DEFAULT gen_random_uuid(),
    email VARCHAR NOT NULL,
    password VARCHAR NOT NULL,
    api_key VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    reset_token VARCHAR,
    reset_sent_at TIMESTAMPTZ,
    email_verification_token VARCHAR,
    email_verification_sent_at TIMESTAMPTZ,
    email_verified_at TIMESTAMPTZ,
    magic_link_token VARCHAR,
    magic_link_expiration TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users (email);
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_pid ON users (pid);
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_api_key ON users (api_key);

CREATE TABLE IF NOT EXISTS profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id),
    name VARCHAR NOT NULL,
    bio TEXT,
    profile_picture VARCHAR,
    images JSONB,
    program VARCHAR,
    gradient_start VARCHAR,
    gradient_end VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_profiles_user_id ON profiles (user_id);

CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id),
    token VARCHAR NOT NULL,
    ip_address VARCHAR,
    user_agent VARCHAR,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions (user_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_token ON sessions (token);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE IF NOT EXISTS user_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id),
    theme VARCHAR NOT NULL,
    language VARCHAR NOT NULL,
    notifications_enabled BOOL NOT NULL,
    privacy_show_program BOOL NOT NULL,
    privacy_discoverable BOOL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_settings_user_id ON user_settings (user_id);

CREATE TABLE IF NOT EXISTS degrees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Tags
-- ============================================================

CREATE TABLE IF NOT EXISTS tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    scope VARCHAR NOT NULL,
    category VARCHAR,
    emoji VARCHAR,
    parent_id UUID REFERENCES tags(id) ON DELETE SET NULL,
    onboarding_order VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tags_scope ON tags (scope);
CREATE INDEX IF NOT EXISTS idx_tags_parent_id ON tags (parent_id);

-- ============================================================
-- Events
-- ============================================================

CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR NOT NULL,
    description TEXT,
    cover_image VARCHAR,
    location VARCHAR,
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ,
    creator_id UUID NOT NULL REFERENCES profiles(id),
    conversation_id VARCHAR,
    latitude FLOAT8,
    longitude FLOAT8,
    max_attendees INTEGER CHECK (max_attendees IS NULL OR max_attendees > 0),
    requires_approval BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_creator_id ON events (creator_id);
CREATE INDEX IF NOT EXISTS idx_events_starts_at ON events (starts_at);

CREATE TABLE IF NOT EXISTS event_attendees (
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    status VARCHAR NOT NULL,
    PRIMARY KEY (event_id, profile_id)
);

CREATE TABLE IF NOT EXISTS event_tags (
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id),
    PRIMARY KEY (event_id, tag_id)
);

CREATE TABLE IF NOT EXISTS profile_tags (
    profile_id UUID NOT NULL REFERENCES profiles(id),
    tag_id UUID NOT NULL REFERENCES tags(id),
    PRIMARY KEY (profile_id, tag_id)
);

CREATE TABLE IF NOT EXISTS event_interactions (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    kind VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, event_id, kind),
    CONSTRAINT event_interactions_kind_check CHECK (kind IN ('saved', 'joined'))
);

CREATE INDEX IF NOT EXISTS idx_event_interactions_event_id ON event_interactions (event_id);

-- ============================================================
-- Uploads
-- ============================================================

CREATE TABLE IF NOT EXISTS uploads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename VARCHAR NOT NULL,
    owner_id UUID REFERENCES profiles(id),
    context VARCHAR NOT NULL,
    context_id VARCHAR,
    mime_type VARCHAR NOT NULL,
    deleted BOOL NOT NULL DEFAULT FALSE,
    thumbhash BYTEA,
    has_variants BOOL NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_uploads_owner_id ON uploads (owner_id);

-- ============================================================
-- Auth helpers
-- ============================================================

CREATE TABLE IF NOT EXISTS auth_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rate_key VARCHAR NOT NULL,
    window_start TIMESTAMPTZ NOT NULL,
    attempts INT4 NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_auth_rate_limits_rate_key ON auth_rate_limits (rate_key);

CREATE TABLE IF NOT EXISTS otp_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR NOT NULL,
    code VARCHAR NOT NULL,
    attempts INT2 NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    last_sent_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_otp_codes_email ON otp_codes (email);
CREATE INDEX IF NOT EXISTS idx_otp_codes_expires_at ON otp_codes (expires_at);

-- ============================================================
-- Job outbox
-- ============================================================

CREATE TABLE IF NOT EXISTS job_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    topic VARCHAR NOT NULL,
    payload JSONB NOT NULL,
    attempts INT4 NOT NULL DEFAULT 0,
    max_attempts INT4 NOT NULL DEFAULT 5,
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    locked_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_job_outbox_available ON job_outbox (available_at) WHERE processed_at IS NULL AND failed_at IS NULL;

ALTER TABLE job_outbox SET (
    autovacuum_vacuum_threshold = 50,
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_threshold = 50
);

-- ============================================================
-- Chat
-- ============================================================

CREATE TABLE IF NOT EXISTS conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind VARCHAR(16) NOT NULL CHECK (kind IN ('dm', 'event')),
    title VARCHAR(255),
    event_id UUID REFERENCES events(id) ON DELETE CASCADE,
    user_low_id INT4 REFERENCES users(id),
    user_high_id INT4 REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_dm_pair
    ON conversations (user_low_id, user_high_id)
    WHERE kind = 'dm';

CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_event
    ON conversations (event_id)
    WHERE kind = 'event';

CREATE TABLE IF NOT EXISTS conversation_members (
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_read_message_id UUID,
    PRIMARY KEY (conversation_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_conversation_members_user ON conversation_members (user_id);

CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    kind VARCHAR(16) NOT NULL DEFAULT 'text' CHECK (kind IN ('text', 'image', 'file')),
    attachment_upload_id UUID REFERENCES uploads(id),
    reply_to_id UUID REFERENCES messages(id) ON DELETE SET NULL,
    client_id VARCHAR(64),
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_created ON messages (conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages (sender_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_client_id
    ON messages (conversation_id, sender_id, client_id)
    WHERE client_id IS NOT NULL AND deleted_at IS NULL;

DO $$ BEGIN
    ALTER TABLE conversation_members
        ADD CONSTRAINT fk_last_read_message
        FOREIGN KEY (last_read_message_id) REFERENCES messages(id) ON DELETE SET NULL;
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

ALTER TABLE messages SET (
    autovacuum_vacuum_threshold = 100,
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_threshold = 100
);

CREATE TABLE IF NOT EXISTS message_reactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji VARCHAR(32) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (message_id, user_id, emoji)
);

CREATE INDEX IF NOT EXISTS idx_message_reactions_message ON message_reactions (message_id);

CREATE TABLE IF NOT EXISTS push_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id VARCHAR(64) NOT NULL,
    ntfy_topic VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, device_id)
);

-- ============================================================
-- Reports & feedback
-- ============================================================

CREATE TABLE IF NOT EXISTS reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    target_type VARCHAR(16) NOT NULL CHECK (target_type IN ('event', 'profile', 'conversation')),
    target_id UUID NOT NULL,
    reason VARCHAR(32) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reports_unique_target
    ON reports (reporter_id, target_type, target_id);

CREATE INDEX IF NOT EXISTS idx_reports_target
    ON reports (target_type, target_id);

CREATE TABLE IF NOT EXISTS recommendation_feedback (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    feedback VARCHAR NOT NULL CHECK (feedback IN ('more', 'less')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, event_id)
);

CREATE INDEX IF NOT EXISTS idx_recommendation_feedback_profile
    ON recommendation_feedback(profile_id);

CREATE TABLE IF NOT EXISTS profile_bookmarks (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    target_profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, target_profile_id),
    CHECK (profile_id <> target_profile_id)
);

CREATE INDEX IF NOT EXISTS idx_profile_bookmarks_profile ON profile_bookmarks(profile_id);

-- ============================================================
-- Full-text search & trigram indexes
-- ============================================================

ALTER TABLE profiles ADD COLUMN IF NOT EXISTS search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(program, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'C')
  ) STORED;

ALTER TABLE profiles ADD COLUMN IF NOT EXISTS public_search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'B')
  ) STORED;

ALTER TABLE events ADD COLUMN IF NOT EXISTS search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(location, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(description, '')), 'C')
  ) STORED;

ALTER TABLE tags ADD COLUMN IF NOT EXISTS search_vector tsvector
  GENERATED ALWAYS AS (to_tsvector('simple', COALESCE(name, ''))) STORED;

CREATE INDEX IF NOT EXISTS idx_profiles_fts ON profiles USING GIN (search_vector);
CREATE INDEX IF NOT EXISTS idx_profiles_public_fts ON profiles USING GIN (public_search_vector);
CREATE INDEX IF NOT EXISTS idx_events_fts ON events USING GIN (search_vector);
CREATE INDEX IF NOT EXISTS idx_tags_fts ON tags USING GIN (search_vector);

CREATE INDEX IF NOT EXISTS idx_profiles_name_trgm ON profiles USING GIN (name gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_profiles_bio_trgm ON profiles USING GIN (bio gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_profiles_program_trgm ON profiles USING GIN (program gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_events_title_trgm ON events USING GIN (title gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_events_description_trgm ON events USING GIN (description gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_events_location_trgm ON events USING GIN (location gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_tags_name_trgm ON tags USING GIN (name gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_events_geo_earth ON events USING GIST (ll_to_earth(latitude, longitude))
  WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

-- ============================================================
-- Triggers
-- ============================================================

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS set_event_interactions_updated_at ON event_interactions;
CREATE TRIGGER set_event_interactions_updated_at
  BEFORE UPDATE ON event_interactions
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- Seed data: interest & event tags
-- ============================================================

-- Root categories
INSERT INTO tags (id, name, scope, category, parent_id) VALUES
  ('b665ff1d-52e3-4efc-9b68-1f53d2efad10', 'Sport', 'interest', 'root', NULL),
  ('a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d', 'Muzyka', 'interest', 'root', NULL),
  ('348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34', 'Sztuka', 'interest', 'root', NULL),
  ('11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8', 'Film i scena', 'interest', 'root', NULL),
  ('20f0febb-cfc4-4b5a-a4a4-140ff8af9abc', 'Technologia', 'interest', 'root', NULL),
  ('7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6', 'Nauka i edukacja', 'interest', 'root', NULL),
  ('3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84', 'Podróże i przygody', 'interest', 'root', NULL),
  ('77d4f8b0-c030-4ed1-8d75-697c15a69f05', 'Kulinaria', 'interest', 'root', NULL),
  ('566e1714-0ec4-4d52-8562-fca84e2c8419', 'Literatura', 'interest', 'root', NULL),
  ('a89488ea-43f1-4c72-94dd-fc3747fb95a0', 'Gry', 'interest', 'root', NULL),
  ('63318021-e21d-4d7d-a4cb-f5e0f15fc833', 'Społeczność', 'interest', 'root', NULL),
  ('460c6106-6f65-4f0d-bbf8-ef49687ec0f3', 'Styl życia', 'interest', 'root', NULL)
ON CONFLICT DO NOTHING;

-- Interest tags (90 curated, deterministic UUIDs)
INSERT INTO tags (id, name, scope, category, onboarding_order, parent_id) VALUES
  ('b009ccfb-ec50-52db-8120-ac7a9f864935', 'Piłka nożna', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('4f8f6849-fbe5-5224-a969-98900f30d527', 'Siatkówka', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('c2d04681-8494-5b60-af0b-56aca95c2dba', 'Koszykówka', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('3d94ed0a-7098-58f2-8a90-20eb6f3594d0', 'Tenis', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('a84a5bec-3203-5add-8def-e90f67c7c981', 'Bieganie', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('23216911-2670-5f63-94cf-5997e506c3b0', 'Pływanie', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('acb75e62-7809-591c-9ecb-fa597914a4ce', 'Siłownia', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('b9975429-50e3-5b82-a3ff-5288a153043c', 'Joga', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('ba4717ed-04bb-5fee-bffe-5d910a6a3cf7', 'Rower', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('2db2d6d9-ab7e-53ca-ad58-40245374fdf5', 'Wspinaczka', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('d222c2a7-9c63-5632-a3fd-79ce3e3a47a2', 'Sporty zimowe', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('e44a6ed5-1403-5443-b08a-10fc08c09094', 'Sztuki walki', 'interest', 'sport', '01', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('617e2bbd-0867-5515-85fd-bdc0b3eb050a', 'Gitara', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('045adbd0-8cbe-5f91-8214-582306f43492', 'Śpiew', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('76dc48f2-cd4f-57f8-9642-f54381e1c5d6', 'Koncerty', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('db322cca-011e-5695-a0bc-b2e0c74a716f', 'Produkcja muzyki', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('ef91805c-e628-56a5-a4e3-7155198f9e05', 'Fortepian', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('bd93b154-4c1b-5d0e-85c5-ba5861134555', 'DJ', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('0936aea4-eb4b-5f6f-a42c-90ddb9f27796', 'Hip-hop', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('7f797a19-af4e-504f-9c64-f9961394ae75', 'Rock', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('e83a0f71-fead-5b24-812e-fa89a58e1445', 'Jazz', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('1017be2b-e367-5812-9f78-7bf1812c576f', 'Muzyka elektroniczna', 'interest', 'muzyka', '02', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('3544ee4a-ed7d-521a-9d95-09b74eee1e18', 'Fotografia', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('83df7b01-4607-500f-b84e-be7c7175212d', 'Malarstwo', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('7a30e8e9-d9d1-522a-815e-57e31bfb7c91', 'Rysunek', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('1050bd14-cacc-535b-880e-8a6a82cbc1aa', 'Grafika', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('f3853923-b53a-5eaa-9855-e8a3fbce4f79', 'Design', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('26459776-02b3-5949-8bc8-2730763f68a7', 'Architektura', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('960280c0-d857-5161-a7d3-bd1db97a0f0a', 'Street art', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('9f77cf6e-c2a2-51d3-af6e-271adacc8161', 'Ceramika', 'interest', 'sztuka', '03', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('47db0957-99f4-5853-a56d-372a652c3f6c', 'Kino', 'interest', 'film', '04', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('a398d56f-1610-534c-8348-1ad2021bda56', 'Seriale', 'interest', 'film', '04', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('6c106fe0-fee3-5cbf-8db1-45a28dd1f41b', 'Teatr', 'interest', 'film', '04', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('e573e23d-a9ae-5152-b397-0ba84c6789d0', 'Stand-up', 'interest', 'film', '04', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('b675d87e-028f-510f-bd5b-750050eeb85c', 'Animacja', 'interest', 'film', '04', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('c6961786-b8c8-5073-8b9d-f95e86ac8a17', 'Film dokumentalny', 'interest', 'film', '04', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('db581004-0cd8-55f7-9f99-d608b2eb312e', 'Programowanie', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('42f94977-7df5-5ca7-82d3-d9a921d41bdf', 'AI i ML', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('02bc359d-3217-533a-bbc4-655b43b91116', 'Cyberbezpieczeństwo', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('6cc1885e-6b69-50de-a7e9-69bfba98d51b', 'Robotyka', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('87c8c825-a424-54f6-968b-d4393f3ffe1c', 'Elektronika', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('cab0ccb7-1f14-5b90-97d9-9ff5ec4fa5ed', 'Gry wideo', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('beca6d47-bdab-5964-81f4-102ea382b96a', 'Startupy', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('05fba6f1-7e04-5200-b933-c7c679fd799c', 'Open source', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('d27a0327-31ed-5018-9c87-7d776783b438', 'Data science', 'interest', 'technologia', '05', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('7e140ecf-3869-5d3c-a500-ed907adfce6d', 'Fizyka', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('c2ca8845-4c65-53f3-9be2-e667655f1121', 'Biologia', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('368c89b3-118a-53d7-bc8b-bb42d11e6abb', 'Chemia', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('88dcb788-0dc7-59ac-bd2d-766ce1a0e1dd', 'Astronomia', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('71dc3564-3c13-5f15-9076-2f0d975d43a3', 'Matematyka', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('9f905329-30aa-5a07-9ce3-ecea8c363d9b', 'Psychologia', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('1872096e-7cda-5df6-a45e-e7e9cb67bd60', 'Filozofia', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('db0582ff-71bb-50c5-8d92-485382572504', 'Lingwistyka', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('f8ae38f9-87f5-57e8-9268-a6e4324b83c9', 'Historia', 'interest', 'nauka', '06', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('e49f7dc2-0811-56f7-b3ee-2560dfb71918', 'Turystyka górska', 'interest', 'podroze', '07', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('71b37402-29ea-5696-8ded-26ebf1978d20', 'Backpacking', 'interest', 'podroze', '07', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('b3f607e8-0f20-5b65-923f-e89d2b260439', 'Camping', 'interest', 'podroze', '07', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('6d43ce80-2564-5e0f-bae9-71d57adc0a1f', 'Podróże zagraniczne', 'interest', 'podroze', '07', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('ea0eed42-a581-531c-abab-b7658a6a774e', 'Zwiedzanie miast', 'interest', 'podroze', '07', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('51bd30b9-26bb-54db-b350-5830ab2c956a', 'Żeglarstwo', 'interest', 'podroze', '07', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('f9399a56-da53-5c96-8d04-2ee25edc57d3', 'Gotowanie', 'interest', 'kulinaria', '08', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('b666d1eb-08f1-5533-b7fc-06d4a83a5fb8', 'Pieczenie', 'interest', 'kulinaria', '08', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('20ba0f8f-c40a-5f22-b438-f1ab7dcf868e', 'Kuchnia azjatycka', 'interest', 'kulinaria', '08', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('f56aae60-0fd5-565d-b765-230ec7ac7fbc', 'Kuchnia włoska', 'interest', 'kulinaria', '08', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('ccadcadf-64be-5602-b600-811925dc1187', 'Wino', 'interest', 'kulinaria', '08', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('e91feae3-7010-5b1b-96db-2be1843db6f7', 'Kawa', 'interest', 'kulinaria', '08', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('98800b59-7c97-5081-8996-a1fb70cb75a7', 'Piwo kraftowe', 'interest', 'kulinaria', '08', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('66e31a90-21d4-5c33-a657-3a29e140c155', 'Beletrystyka', 'interest', 'literatura', '09', '566e1714-0ec4-4d52-8562-fca84e2c8419'),
  ('9a638968-443f-53a7-83aa-29fdd109ec73', 'Fantasy i sci-fi', 'interest', 'literatura', '09', '566e1714-0ec4-4d52-8562-fca84e2c8419'),
  ('9b7b02f9-0a4a-55ba-a96b-f7cf95df4301', 'Klub książki', 'interest', 'literatura', '09', '566e1714-0ec4-4d52-8562-fca84e2c8419'),
  ('d808862c-651f-598d-92a3-fdb826c02703', 'Poezja', 'interest', 'literatura', '09', '566e1714-0ec4-4d52-8562-fca84e2c8419'),
  ('a68fd876-3921-531d-abb9-a534c9ae3b07', 'Pisarstwo', 'interest', 'literatura', '09', '566e1714-0ec4-4d52-8562-fca84e2c8419'),
  ('5bd831a9-724a-5e0b-ab5e-cde71d40aa58', 'Gry planszowe', 'interest', 'gry', '10', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('724924d2-04c5-5ebc-a0ae-03d9dd8f3dfa', 'Gry karciane', 'interest', 'gry', '10', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('81f665a7-9811-5206-a161-fb68ab4fd45d', 'Szachy', 'interest', 'gry', '10', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('3ca81a60-1cc6-5cfd-876b-35348f82b57b', 'Escape room', 'interest', 'gry', '10', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('6a64142c-2196-5c9f-8cd8-ab29fd39d400', 'RPG', 'interest', 'gry', '10', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('80d2e431-1e08-5250-bf8f-8afc3649a642', 'E-sport', 'interest', 'gry', '10', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('edc67245-3596-538a-aa57-d2526cfceb57', 'Wolontariat', 'interest', 'spolecznosc', '11', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('1ebc1260-2a5e-5c94-85fb-f0020ffe212b', 'Ekologia', 'interest', 'spolecznosc', '11', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('0b969f83-bf34-5522-9f01-05fdd7579bb2', 'Debaty', 'interest', 'spolecznosc', '11', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('378a8a7b-4e14-5958-b42c-bd37828a2595', 'Polityka', 'interest', 'spolecznosc', '11', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('817fd474-ea5a-5614-9ed8-af9bb82f694a', 'Organizacja wydarzeń', 'interest', 'spolecznosc', '11', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('168b4455-fd52-5ba5-9fa0-5ebb4f3448f5', 'NGO', 'interest', 'spolecznosc', '11', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('b576c7ff-2038-5530-80e5-e313ebfd475c', 'Medytacja', 'interest', 'styl_zycia', '12', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'),
  ('d4c88ac5-9564-54a5-b368-bf0dc069f466', 'Zdrowe odżywianie', 'interest', 'styl_zycia', '12', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'),
  ('29b5d222-4e01-53c3-b38f-2556f404be3c', 'Minimalizm', 'interest', 'styl_zycia', '12', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'),
  ('de248dd4-203d-5408-b731-f534023c8deb', 'Moda', 'interest', 'styl_zycia', '12', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'),
  ('f4b20bdc-a473-5b6c-a38c-d7ca291ffa38', 'Taniec', 'interest', 'styl_zycia', '12', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'),
  ('92ac54b5-f45e-5eef-9e01-4b78bf3802ce', 'Fitness outdoorowy', 'interest', 'styl_zycia', '12', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3')
ON CONFLICT DO NOTHING;

-- Event tags
INSERT INTO tags (id, name, scope, category, parent_id) VALUES
  ('f77d23b0-28bf-4db0-82bf-1efd66f8244e', 'Bieg grupowy', 'event', 'sport', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('f8398c10-ea1a-4b8d-a33f-80c91bb4270f', 'Sesja jogi', 'event', 'sport', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('00d91f4a-f275-4744-b08b-f396f0e841b1', 'Mecz amatorski', 'event', 'sport', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('f057fe98-62c5-4c54-bfea-afc2cc74e07d', 'Jam session', 'event', 'muzyka', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('2f61bc8f-54d7-4962-b468-4b3abf9b7626', 'Koncert na żywo', 'event', 'muzyka', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('153b6c59-e6de-453f-a6df-f52132d1f77c', 'Wystawa', 'event', 'sztuka', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('636bc541-8282-4255-b0b8-a3a72e738932', 'Warsztaty kreatywne', 'event', 'sztuka', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('8ae53725-2867-48ac-925c-e97cfed87aa7', 'Wieczór filmowy', 'event', 'film', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('67de9126-c32b-43f1-a84a-88142cd0eac9', 'Stand-up na żywo', 'event', 'film', '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'),
  ('6e605dd1-411a-4b42-b616-64221c1c9768', 'Hackathon', 'event', 'technologia', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('e574bb25-9808-4d40-9607-200f70c72ad9', 'Meetup technologiczny', 'event', 'technologia', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('f14ac9dc-8623-4971-8f0f-5b82e80a2e8f', 'Koło naukowe', 'event', 'nauka', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('af14d14c-fe6e-48c6-a76c-9c2226051381', 'Grupa nauki', 'event', 'nauka', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('f2bc5778-aeb2-4343-a2b6-d1e818c15b31', 'Spacer miejski', 'event', 'podroze', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('1cafdf26-13de-4bdb-acb8-bb2682dd2bf8', 'Wyjazd outdoorowy', 'event', 'podroze', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('df9cd37d-3055-4d17-ac32-0c6b860df06f', 'Degustacja', 'event', 'kulinaria', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('6d2fb7b3-1c72-4c13-98bd-f5f48b53c7cb', 'Warsztaty kulinarne', 'event', 'kulinaria', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('f1cc04b8-1c1d-4556-b6fb-8e7b4ac5e376', 'Klub książki', 'event', 'literatura', '566e1714-0ec4-4d52-8562-fca84e2c8419'),
  ('5b75dc99-181d-404a-a5dc-3b15c7e25db5', 'Pisanie razem', 'event', 'literatura', '566e1714-0ec4-4d52-8562-fca84e2c8419'),
  ('d01991c9-00f2-49ab-94f2-c568ec336f42', 'Wieczór planszówek', 'event', 'gry', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('4217f53b-ec49-423d-9b96-3d41d0e31d64', 'Sesja RPG', 'event', 'gry', 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'),
  ('a2cdb278-fb19-48b8-8438-8110afcbdf1f', 'Wolontariat', 'event', 'spolecznosc', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('4586b307-c167-456d-abd2-0d3ef65afdc4', 'Debata', 'event', 'spolecznosc', '63318021-e21d-4d7d-a4cb-f5e0f15fc833'),
  ('182039cc-3ae5-4953-a2a1-b9b1135cd1c1', 'Krąg medytacji', 'event', 'styl_zycia', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'),
  ('b9c896e9-5d49-4847-8c58-9ed77efc5f0d', 'Warsztaty tańca', 'event', 'styl_zycia', '460c6106-6f65-4f0d-bbf8-ef49687ec0f3')
ON CONFLICT DO NOTHING;
