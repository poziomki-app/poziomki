-- Initial schema migration for Poziomki backend
-- Matches backend/src/db/schema.rs exactly

CREATE TABLE users (
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

CREATE UNIQUE INDEX idx_users_email ON users (email);
CREATE UNIQUE INDEX idx_users_pid ON users (pid);
CREATE UNIQUE INDEX idx_users_api_key ON users (api_key);

CREATE TABLE profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id),
    name VARCHAR NOT NULL,
    bio TEXT,
    age INT2 NOT NULL,
    profile_picture VARCHAR,
    images JSONB,
    program VARCHAR,
    gradient_start VARCHAR,
    gradient_end VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_profiles_user_id ON profiles (user_id);

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id),
    token VARCHAR NOT NULL,
    ip_address VARCHAR,
    user_agent VARCHAR,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE UNIQUE INDEX idx_sessions_token ON sessions (token);

CREATE TABLE user_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT4 NOT NULL REFERENCES users(id),
    theme VARCHAR NOT NULL,
    language VARCHAR NOT NULL,
    notifications_enabled BOOL NOT NULL,
    privacy_show_age BOOL NOT NULL,
    privacy_show_program BOOL NOT NULL,
    privacy_discoverable BOOL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_user_settings_user_id ON user_settings (user_id);

CREATE TABLE degrees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL,
    scope VARCHAR NOT NULL,
    category VARCHAR,
    emoji VARCHAR,
    onboarding_order VARCHAR,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tags_scope ON tags (scope);

CREATE TABLE events (
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_events_creator_id ON events (creator_id);
CREATE INDEX idx_events_starts_at ON events (starts_at);

CREATE TABLE event_attendees (
    event_id UUID NOT NULL REFERENCES events(id),
    profile_id UUID NOT NULL REFERENCES profiles(id),
    status VARCHAR NOT NULL,
    PRIMARY KEY (event_id, profile_id)
);

CREATE TABLE event_tags (
    event_id UUID NOT NULL REFERENCES events(id),
    tag_id UUID NOT NULL REFERENCES tags(id),
    PRIMARY KEY (event_id, tag_id)
);

CREATE TABLE profile_tags (
    profile_id UUID NOT NULL REFERENCES profiles(id),
    tag_id UUID NOT NULL REFERENCES tags(id),
    PRIMARY KEY (profile_id, tag_id)
);

CREATE TABLE uploads (
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

CREATE INDEX idx_uploads_owner_id ON uploads (owner_id);

CREATE TABLE auth_rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rate_key VARCHAR NOT NULL,
    window_start TIMESTAMPTZ NOT NULL,
    attempts INT4 NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_auth_rate_limits_rate_key ON auth_rate_limits (rate_key);

CREATE TABLE otp_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR NOT NULL,
    code VARCHAR NOT NULL,
    attempts INT2 NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    last_sent_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_otp_codes_email ON otp_codes (email);

CREATE TABLE job_outbox (
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

CREATE INDEX idx_job_outbox_available ON job_outbox (available_at) WHERE processed_at IS NULL AND failed_at IS NULL;

CREATE TABLE matrix_dm_rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_low_pid UUID NOT NULL REFERENCES profiles(id),
    user_high_pid UUID NOT NULL REFERENCES profiles(id),
    room_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_matrix_dm_rooms_pair ON matrix_dm_rooms (user_low_pid, user_high_pid);
CREATE UNIQUE INDEX idx_matrix_dm_rooms_room_id ON matrix_dm_rooms (room_id);

-- Extensions for search and geo
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS cube;
CREATE EXTENSION IF NOT EXISTS earthdistance;

-- Full-text search: tsvector generated columns
ALTER TABLE profiles ADD COLUMN search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(program, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'C')
  ) STORED;

ALTER TABLE events ADD COLUMN search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(location, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(description, '')), 'C')
  ) STORED;

-- GIN indexes for full-text search
CREATE INDEX idx_profiles_fts ON profiles USING GIN (search_vector);
CREATE INDEX idx_events_fts ON events USING GIN (search_vector);

-- Trigram indexes for LIKE fallback
CREATE INDEX idx_profiles_name_trgm ON profiles USING GIN (name gin_trgm_ops);
CREATE INDEX idx_profiles_bio_trgm ON profiles USING GIN (bio gin_trgm_ops);
CREATE INDEX idx_profiles_program_trgm ON profiles USING GIN (program gin_trgm_ops);
CREATE INDEX idx_events_title_trgm ON events USING GIN (title gin_trgm_ops);
CREATE INDEX idx_events_description_trgm ON events USING GIN (description gin_trgm_ops);
CREATE INDEX idx_events_location_trgm ON events USING GIN (location gin_trgm_ops);
CREATE INDEX idx_tags_name_trgm ON tags USING GIN (name gin_trgm_ops);

-- Geo index for earthdistance queries
CREATE INDEX idx_events_geo_earth ON events USING GIST (ll_to_earth(latitude, longitude))
  WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

-- Autovacuum tuning for high-churn outbox table
ALTER TABLE job_outbox SET (
  autovacuum_vacuum_threshold = 50,
  autovacuum_vacuum_scale_factor = 0.05,
  autovacuum_analyze_threshold = 50
);
