-- Flag users that exist solely to support Google Play reviewer QA. Profiles
-- and DMs owned by these accounts are visible only between other stub users,
-- so they never leak into normal users' matching feed or chat list.
ALTER TABLE users ADD COLUMN is_review_stub BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX users_is_review_stub_idx ON users (is_review_stub) WHERE is_review_stub = true;
