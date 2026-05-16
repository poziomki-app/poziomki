DROP POLICY profiles_viewer ON public.profiles;
CREATE POLICY profiles_viewer ON public.profiles
    FOR SELECT TO poziomki_api
    USING (id IN (SELECT id FROM app.profiles_in_current_bucket()));
