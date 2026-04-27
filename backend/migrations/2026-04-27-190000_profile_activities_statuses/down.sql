DELETE FROM public.profile_tags
WHERE tag_id IN (
    '7b01dd2e-01ff-501e-bba5-e75f8ee3d4fa',
    'c7104ef3-a7fe-5eb0-b0eb-542c1f46618a',
    '89f5603c-9974-5e2c-8b76-21097f40e6b0',
    '45d64e17-4069-5db5-a4a4-8f6c3aa79908',
    'e63fb3e4-a029-5ae2-b33c-9e5b99f765d2',
    '934d8002-cea6-524f-aab0-dae497fbf067',
    'da79350b-bf17-5c7b-8d2f-7294229f53f4',
    '77ed4d70-4523-525e-aced-e2498dda21b3',
    'aac2fb77-2ab8-5e47-b930-1610ba7cb462',
    'e83e83b7-1a35-503f-b8e6-1355afc16ff0',
    '2b42b1c9-35ae-50a8-8d43-7487f67684c5',
    '19889079-09b1-5a5e-981d-d8ff5000b4f8',
    '869196ff-32cf-57d3-9997-a06070b5cfbb',
    '05126d48-de28-5e2c-a9a5-a78934b7dd1c'
);

DELETE FROM public.tags
WHERE id IN (
    '7b01dd2e-01ff-501e-bba5-e75f8ee3d4fa',
    'c7104ef3-a7fe-5eb0-b0eb-542c1f46618a',
    '89f5603c-9974-5e2c-8b76-21097f40e6b0',
    '45d64e17-4069-5db5-a4a4-8f6c3aa79908',
    'e63fb3e4-a029-5ae2-b33c-9e5b99f765d2',
    '934d8002-cea6-524f-aab0-dae497fbf067',
    'da79350b-bf17-5c7b-8d2f-7294229f53f4',
    '77ed4d70-4523-525e-aced-e2498dda21b3',
    'aac2fb77-2ab8-5e47-b930-1610ba7cb462',
    'e83e83b7-1a35-503f-b8e6-1355afc16ff0',
    '2b42b1c9-35ae-50a8-8d43-7487f67684c5',
    '19889079-09b1-5a5e-981d-d8ff5000b4f8',
    '869196ff-32cf-57d3-9997-a06070b5cfbb',
    '05126d48-de28-5e2c-a9a5-a78934b7dd1c'
);

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

ALTER TABLE public.profiles
    DROP CONSTRAINT IF EXISTS profiles_status_text_length,
    DROP COLUMN IF EXISTS status_text;
