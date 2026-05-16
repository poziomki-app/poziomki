-- Allow the viewer to SELECT their own profile rows directly, in addition to
-- bucket reads. The previous policy went solely through
-- `app.profiles_in_current_bucket()`, which is STABLE and therefore evaluates
-- against the statement's pre-INSERT snapshot. That broke `INSERT ... RETURNING`
-- on the very first profile a user creates: the new row passed the INSERT
-- WITH CHECK but couldn't be seen by the implicit SELECT used to populate
-- RETURNING, so onboarding "create profile" failed with a misleading
-- "new row violates row-level security policy" error.

DROP POLICY profiles_viewer ON public.profiles;
CREATE POLICY profiles_viewer ON public.profiles
    FOR SELECT TO poziomki_api
    USING (
        user_id = app.current_user_id()
        OR id IN (SELECT id FROM app.profiles_in_current_bucket())
    );
