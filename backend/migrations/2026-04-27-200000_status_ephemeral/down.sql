DROP INDEX IF EXISTS idx_profiles_status_expires_at;

DROP INDEX IF EXISTS idx_profiles_fts;
DROP INDEX IF EXISTS idx_profiles_public_fts;

ALTER TABLE public.profiles
    DROP COLUMN IF EXISTS search_vector,
    DROP COLUMN IF EXISTS public_search_vector;

ALTER TABLE public.profiles ADD COLUMN search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(program, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(status_text, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'C')
  ) STORED;

ALTER TABLE public.profiles ADD COLUMN public_search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(status_text, '')), 'B') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'B')
  ) STORED;

CREATE INDEX IF NOT EXISTS idx_profiles_fts ON public.profiles USING GIN (search_vector);
CREATE INDEX IF NOT EXISTS idx_profiles_public_fts ON public.profiles USING GIN (public_search_vector);

ALTER TABLE public.profiles
    DROP CONSTRAINT IF EXISTS profiles_status_emoji_length,
    DROP COLUMN IF EXISTS status_emoji,
    DROP COLUMN IF EXISTS status_expires_at;
