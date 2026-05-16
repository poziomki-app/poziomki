DROP POLICY uploads_update ON public.uploads;
CREATE POLICY uploads_update ON public.uploads
    FOR UPDATE TO poziomki_api
    USING (owner_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (owner_id IN (SELECT id FROM app.viewer_profile_ids()));
