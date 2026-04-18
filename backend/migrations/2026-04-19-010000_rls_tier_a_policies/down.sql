-- Drop Tier A policies + disable RLS. `DROP POLICY` is idempotent only
-- via `IF EXISTS`; `ALTER TABLE ... DISABLE ROW LEVEL SECURITY` is safe
-- whether or not the table currently has RLS enabled.
DROP POLICY IF EXISTS reports_viewer ON public.reports;
DROP POLICY IF EXISTS event_interactions_viewer ON public.event_interactions;
DROP POLICY IF EXISTS recommendation_feedback_viewer ON public.recommendation_feedback;
DROP POLICY IF EXISTS profile_blocks_viewer ON public.profile_blocks;
DROP POLICY IF EXISTS profile_bookmarks_viewer ON public.profile_bookmarks;
DROP POLICY IF EXISTS task_completions_viewer ON public.task_completions;
DROP POLICY IF EXISTS xp_scans_viewer ON public.xp_scans;
DROP POLICY IF EXISTS push_subscriptions_viewer ON public.push_subscriptions;
DROP POLICY IF EXISTS user_audit_log_viewer ON public.user_audit_log;
DROP POLICY IF EXISTS user_settings_viewer ON public.user_settings;
DROP POLICY IF EXISTS sessions_viewer ON public.sessions;
DROP POLICY IF EXISTS profile_tags_viewer ON public.profile_tags;
DROP POLICY IF EXISTS profiles_viewer ON public.profiles;
DROP POLICY IF EXISTS users_viewer ON public.users;

ALTER TABLE public.reports NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.reports DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.event_interactions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.event_interactions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.recommendation_feedback NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.recommendation_feedback DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.profile_blocks NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.profile_blocks DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.profile_bookmarks NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.profile_bookmarks DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.task_completions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.task_completions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.xp_scans NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.xp_scans DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.push_subscriptions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.push_subscriptions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_audit_log NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.user_audit_log DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_settings NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.user_settings DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.sessions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.sessions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.profile_tags NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.profile_tags DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.profiles NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.profiles DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.users NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.users DISABLE ROW LEVEL SECURITY;

DROP FUNCTION IF EXISTS app.profiles_in_current_bucket();
DROP FUNCTION IF EXISTS app.viewer_profile_ids();
DROP FUNCTION IF EXISTS app.current_is_stub();
DROP FUNCTION IF EXISTS app.current_user_id();
