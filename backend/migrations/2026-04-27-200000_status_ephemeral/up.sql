-- Ephemeral profile status (vibe). 24h TTL, set from the Poznaj
-- composer rather than profile edit.
--
-- `status_text` already exists from the previous migration. Here we
-- add the emoji prefix and an expiry timestamp. Reads must filter
-- on `status_expires_at > now()`; rows past expiry are still in the
-- table until the daily sweep clears them.

ALTER TABLE public.profiles
    ADD COLUMN IF NOT EXISTS status_emoji TEXT,
    ADD COLUMN IF NOT EXISTS status_expires_at TIMESTAMPTZ;

ALTER TABLE public.profiles
    DROP CONSTRAINT IF EXISTS profiles_status_emoji_length;

-- Single grapheme cluster is hard to enforce in pure SQL; cap byte
-- length so we can't be filled with a 1KB ZWJ-joined sequence.
-- Most legit emoji (incl. ZWJ + skin tone) fit comfortably under 32 bytes.
ALTER TABLE public.profiles
    ADD CONSTRAINT profiles_status_emoji_length
    CHECK (status_emoji IS NULL OR octet_length(status_emoji) <= 32);

-- Index supporting the daily sweep job (and any read-time filter
-- that scans for live status). Partial — only rows with a status
-- carry an expiry; the rest don't need indexing.
CREATE INDEX IF NOT EXISTS idx_profiles_status_expires_at
    ON public.profiles (status_expires_at)
    WHERE status_expires_at IS NOT NULL;

-- NOTE: `status_text` is still part of `search_vector` /
-- `public_search_vector` from migration 190000. Now that status is
-- ephemeral, we'd ideally drop it from the FTS columns — but that
-- requires DROP/ADD on a STORED generated column, which rewrites
-- the table and rebuilds two GIN indexes non-CONCURRENTLY inside a
-- single Diesel transaction. Skipped here to keep this deploy
-- non-blocking; expired status_text matches are masked at read
-- time by the `status_expires_at > now()` filter on every read
-- path, so the worst case is a profile surfacing in search whose
-- status pill is no longer rendered. Follow-up: swap FTS columns
-- in a dedicated non-transactional migration using
-- CREATE INDEX CONCURRENTLY.
