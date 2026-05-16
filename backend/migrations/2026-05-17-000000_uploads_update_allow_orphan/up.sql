-- Allow the viewer to UPDATE one of their own uploads to owner_id = NULL.
-- The delete-account flow needs this to orphan uploads before deleting the
-- profile cascade (the immutability trigger already permits this transition).
-- The USING clause still forces the old row to be the viewer's own; only
-- WITH CHECK is relaxed to also accept a NULL'd new row.

DROP POLICY uploads_update ON public.uploads;
CREATE POLICY uploads_update ON public.uploads
    FOR UPDATE TO poziomki_api
    USING (owner_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (
        owner_id IS NULL
        OR owner_id IN (SELECT id FROM app.viewer_profile_ids())
    );
