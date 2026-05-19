DROP FUNCTION IF EXISTS app.finalize_pre_launch_profile(INT);
DROP INDEX IF EXISTS public.idx_profiles_pre_launch;
ALTER TABLE public.profiles DROP COLUMN IF EXISTS is_pre_launch;
