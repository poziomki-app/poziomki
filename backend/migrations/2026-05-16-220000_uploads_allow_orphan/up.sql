-- Allow owner_id to transition to NULL so the delete-account flow can
-- orphan a user's uploads before deleting the profile row. The original
-- threat — a viewer "adopting" anon uploads or reassigning to another
-- profile they own — still requires NEW.owner_id to be non-NULL, so
-- this remains blocked. Going to NULL is a pure orphaning operation
-- (no row is gained, none changes hands).

CREATE OR REPLACE FUNCTION app.reject_uploads_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.id IS DISTINCT FROM OLD.id THEN
        RAISE EXCEPTION 'uploads.id is immutable';
    END IF;
    IF NEW.owner_id IS DISTINCT FROM OLD.owner_id AND NEW.owner_id IS NOT NULL THEN
        RAISE EXCEPTION 'uploads.owner_id can only be cleared to NULL';
    END IF;
    RETURN NEW;
END
$$;
