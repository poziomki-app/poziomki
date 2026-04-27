-- Ephemeral profile status (vibe). 24h TTL, set from the Poznaj
-- composer rather than profile edit.
--
-- `status_text` already exists from the previous migration. Here we
-- add the emoji prefix and an expiry timestamp. Reads must filter
-- on `status_expires_at > now()`; rows past expiry are still in the
-- table until the daily sweep clears them.
--
-- We also strip `status_text` from the FTS regen path. There's no
-- point indexing 24h ephemera, and search hits on stale text would
-- be confusing.

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

-- Refresh generated search vectors to drop status_text — it's ephemeral
-- and shouldn't be indexed.
DROP INDEX IF EXISTS idx_profiles_fts;
DROP INDEX IF EXISTS idx_profiles_public_fts;

ALTER TABLE public.profiles
    DROP COLUMN IF EXISTS search_vector,
    DROP COLUMN IF EXISTS public_search_vector;

ALTER TABLE public.profiles ADD COLUMN search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(program, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'C')
  ) STORED;

ALTER TABLE public.profiles ADD COLUMN public_search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'B')
  ) STORED;

CREATE INDEX IF NOT EXISTS idx_profiles_fts ON public.profiles USING GIN (search_vector);
CREATE INDEX IF NOT EXISTS idx_profiles_public_fts ON public.profiles USING GIN (public_search_vector);

-- Index supporting the daily sweep job (and any read-time filter
-- that scans for live status). Partial — only rows with a status
-- carry an expiry; the rest don't need indexing.
CREATE INDEX IF NOT EXISTS idx_profiles_status_expires_at
    ON public.profiles (status_expires_at)
    WHERE status_expires_at IS NOT NULL;
