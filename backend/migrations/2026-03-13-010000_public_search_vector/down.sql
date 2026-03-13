DROP INDEX IF EXISTS idx_profiles_public_fts;
ALTER TABLE profiles DROP COLUMN IF EXISTS public_search_vector;
