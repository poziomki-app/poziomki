-- Tier A: enable RLS on every per-user / per-profile table and attach
-- "own row only" policies (with a same-stub-bucket relaxation for the
-- profile-lookup tables that matching + attendee listing require).
--
-- Every handler has already been wrapped in `db::with_viewer_tx` in PRs
-- #3-#8, so `app.user_id` + `app.is_stub` are set on every API request
-- before any query touches these tables. This migration adds the
-- policies that enforce those GUCs at the DB layer.
--
-- Roles:
-- * poziomki_api — NOBYPASSRLS; the running API. Policies bite here.
-- * poziomki_worker — BYPASSRLS; background jobs that legitimately act
--   cross-user (outbox dispatch, variant generation, etc). Unaffected.
-- * poziomki (owner / migrations) — superuser in every environment we
--   operate, so it bypasses RLS regardless of `FORCE ROW LEVEL SECURITY`.
--   Migrations and test fixtures keep working.

-- ---------------------------------------------------------------------------
-- Helper functions consumed by the policies below.
--
-- `current_user_id()` / `current_is_stub()` surface the GUCs that
-- `with_viewer_tx` emits via `SET LOCAL app.user_id = '…' / app.is_stub
-- = '…'`. Missing GUC → NULL / false; `NULL = id` never matches, so the
-- anon case falls through to "sees nothing".
--
-- `viewer_profile_ids()` is a pure convenience: the same
-- `(SELECT id FROM profiles WHERE user_id = …)` subquery would
-- otherwise repeat across every policy. Regular (non-SD) function so
-- it still runs under the caller's privileges and RLS; the profiles
-- policy below explicitly allows the viewer to see their own row.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION app.current_user_id()
RETURNS int
LANGUAGE sql
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT NULLIF(current_setting('app.user_id', true), '')::int
$$;

COMMENT ON FUNCTION app.current_user_id() IS
    'Viewer user_id from the app.user_id GUC set by with_viewer_tx. NULL when unset — policies using `= current_user_id()` then match nothing.';

CREATE OR REPLACE FUNCTION app.current_is_stub()
RETURNS bool
LANGUAGE sql
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT COALESCE(NULLIF(current_setting('app.is_stub', true), ''), 'false')::bool
$$;

COMMENT ON FUNCTION app.current_is_stub() IS
    'Viewer stub flag from the app.is_stub GUC. Defaults to false; review-stub and real-user buckets never mix.';

CREATE OR REPLACE FUNCTION app.viewer_profile_ids()
RETURNS TABLE (id uuid)
LANGUAGE sql
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT id FROM public.profiles WHERE user_id = app.current_user_id()
$$;

COMMENT ON FUNCTION app.viewer_profile_ids() IS
    'Profile ids owned by the current viewer. Consumed by policies that scope access to "profiles I own" (profile_tags, profile_bookmarks, profile_blocks, xp_scans, task_completions, recommendation_feedback, event_interactions, reports).';

REVOKE EXECUTE ON FUNCTION app.current_user_id() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.current_is_stub() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.viewer_profile_ids() FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.current_user_id() TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.current_is_stub() TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_profile_ids() TO poziomki_api';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- users
--
-- Own row only AND matching stub-bucket. WITH CHECK is narrower than
-- USING because INSERTs happen only via `app.create_user_for_signup`
-- (SECURITY DEFINER, bypasses RLS); the API role should never insert
-- directly, so WITH CHECK is effectively a belt-and-suspenders.
-- ---------------------------------------------------------------------------
ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.users FORCE ROW LEVEL SECURITY;
CREATE POLICY users_viewer ON public.users
    FOR ALL TO poziomki_api
    USING (id = app.current_user_id() AND is_review_stub = app.current_is_stub())
    WITH CHECK (id = app.current_user_id());

-- ---------------------------------------------------------------------------
-- profiles
--
-- Cross-viewer reads within the same stub bucket so matching, attendee
-- previews, DM member resolution, etc. can continue to read other
-- users' rows. WITH CHECK is own-profile-only so a viewer can't create
-- or mutate someone else's profile.
-- ---------------------------------------------------------------------------
ALTER TABLE public.profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.profiles FORCE ROW LEVEL SECURITY;
CREATE POLICY profiles_viewer ON public.profiles
    FOR ALL TO poziomki_api
    USING (
        EXISTS (
            SELECT 1 FROM public.users u
            WHERE u.id = profiles.user_id
              AND u.is_review_stub = app.current_is_stub()
        )
    )
    WITH CHECK (user_id = app.current_user_id());

-- ---------------------------------------------------------------------------
-- profile_tags
--
-- Same bucket for reads (profile interest lookups drive matching /
-- onboarding / export). Writes restricted to the viewer's own
-- profile.
-- ---------------------------------------------------------------------------
ALTER TABLE public.profile_tags ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.profile_tags FORCE ROW LEVEL SECURITY;
CREATE POLICY profile_tags_viewer ON public.profile_tags
    FOR ALL TO poziomki_api
    USING (
        profile_id IN (
            SELECT p.id FROM public.profiles p
            JOIN public.users u ON u.id = p.user_id
            WHERE u.is_review_stub = app.current_is_stub()
        )
    )
    WITH CHECK (profile_id IN (SELECT id FROM app.viewer_profile_ids()));

-- ---------------------------------------------------------------------------
-- sessions
--
-- Own user_id only. Authentication reads (`app.resolve_session`) run
-- SECURITY DEFINER and bypass RLS by design — this policy gates the
-- authenticated code paths that list / invalidate / audit sessions.
-- ---------------------------------------------------------------------------
ALTER TABLE public.sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.sessions FORCE ROW LEVEL SECURITY;
CREATE POLICY sessions_viewer ON public.sessions
    FOR ALL TO poziomki_api
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());

-- ---------------------------------------------------------------------------
-- user_settings
-- ---------------------------------------------------------------------------
ALTER TABLE public.user_settings ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_settings FORCE ROW LEVEL SECURITY;
CREATE POLICY user_settings_viewer ON public.user_settings
    FOR ALL TO poziomki_api
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());

-- ---------------------------------------------------------------------------
-- user_audit_log
--
-- Column is `user_pid uuid`, so the policy resolves the viewer's pid
-- via a one-row subquery on users (already same-row-scoped via the
-- users policy above).
-- ---------------------------------------------------------------------------
ALTER TABLE public.user_audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_audit_log FORCE ROW LEVEL SECURITY;
CREATE POLICY user_audit_log_viewer ON public.user_audit_log
    FOR ALL TO poziomki_api
    USING (user_pid = (SELECT pid FROM public.users WHERE id = app.current_user_id()))
    WITH CHECK (user_pid = (SELECT pid FROM public.users WHERE id = app.current_user_id()));

-- ---------------------------------------------------------------------------
-- push_subscriptions
-- ---------------------------------------------------------------------------
ALTER TABLE public.push_subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.push_subscriptions FORCE ROW LEVEL SECURITY;
CREATE POLICY push_subscriptions_viewer ON public.push_subscriptions
    FOR ALL TO poziomki_api
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());

-- ---------------------------------------------------------------------------
-- xp_scans
--
-- Scanner owns the row. Viewing your own scan history; filing a scan
-- record requires that you're the scanner (not the scanned).
-- ---------------------------------------------------------------------------
ALTER TABLE public.xp_scans ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.xp_scans FORCE ROW LEVEL SECURITY;
CREATE POLICY xp_scans_viewer ON public.xp_scans
    FOR ALL TO poziomki_api
    USING (scanner_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (scanner_id IN (SELECT id FROM app.viewer_profile_ids()));

-- ---------------------------------------------------------------------------
-- task_completions
-- ---------------------------------------------------------------------------
ALTER TABLE public.task_completions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.task_completions FORCE ROW LEVEL SECURITY;
CREATE POLICY task_completions_viewer ON public.task_completions
    FOR ALL TO poziomki_api
    USING (profile_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (profile_id IN (SELECT id FROM app.viewer_profile_ids()));

-- ---------------------------------------------------------------------------
-- profile_bookmarks
--
-- `profile_id` is the bookmarker; `target_profile_id` is whom they
-- saved. Policy scopes to "bookmarks I made" for both reads and writes.
-- ---------------------------------------------------------------------------
ALTER TABLE public.profile_bookmarks ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.profile_bookmarks FORCE ROW LEVEL SECURITY;
CREATE POLICY profile_bookmarks_viewer ON public.profile_bookmarks
    FOR ALL TO poziomki_api
    USING (profile_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (profile_id IN (SELECT id FROM app.viewer_profile_ids()));

-- ---------------------------------------------------------------------------
-- profile_blocks
--
-- Chat reads blocks in both directions — it needs to know "A blocked
-- B" AND "B blocked A" to gate a DM. Writes are always initiated by
-- the blocker.
-- ---------------------------------------------------------------------------
ALTER TABLE public.profile_blocks ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.profile_blocks FORCE ROW LEVEL SECURITY;
CREATE POLICY profile_blocks_viewer ON public.profile_blocks
    FOR ALL TO poziomki_api
    USING (
        blocker_id IN (SELECT id FROM app.viewer_profile_ids())
        OR blocked_id IN (SELECT id FROM app.viewer_profile_ids())
    )
    WITH CHECK (blocker_id IN (SELECT id FROM app.viewer_profile_ids()));

-- ---------------------------------------------------------------------------
-- recommendation_feedback
-- ---------------------------------------------------------------------------
ALTER TABLE public.recommendation_feedback ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.recommendation_feedback FORCE ROW LEVEL SECURITY;
CREATE POLICY recommendation_feedback_viewer ON public.recommendation_feedback
    FOR ALL TO poziomki_api
    USING (profile_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (profile_id IN (SELECT id FROM app.viewer_profile_ids()));

-- ---------------------------------------------------------------------------
-- event_interactions
--
-- Personal event state (saved / joined). Used by export + matching
-- exclusion sets; always viewer-scoped.
-- ---------------------------------------------------------------------------
ALTER TABLE public.event_interactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.event_interactions FORCE ROW LEVEL SECURITY;
CREATE POLICY event_interactions_viewer ON public.event_interactions
    FOR ALL TO poziomki_api
    USING (profile_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (profile_id IN (SELECT id FROM app.viewer_profile_ids()));

-- ---------------------------------------------------------------------------
-- reports
--
-- Reporter owns the row. Moderator / admin reads will arrive later via
-- a dedicated DB role or SECURITY DEFINER path, not via anon.
-- ---------------------------------------------------------------------------
ALTER TABLE public.reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.reports FORCE ROW LEVEL SECURITY;
CREATE POLICY reports_viewer ON public.reports
    FOR ALL TO poziomki_api
    USING (reporter_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (reporter_id IN (SELECT id FROM app.viewer_profile_ids()));
