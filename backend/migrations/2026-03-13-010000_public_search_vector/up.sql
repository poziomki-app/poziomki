ALTER TABLE profiles ADD COLUMN public_search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('simple', COALESCE(name, '')), 'A') ||
    setweight(to_tsvector('simple', COALESCE(bio, '')), 'C')
  ) STORED;

CREATE INDEX idx_profiles_public_fts ON profiles USING GIN (public_search_vector);
