CREATE OR REPLACE FUNCTION app.reject_uploads_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.owner_id IS DISTINCT FROM OLD.owner_id THEN
        RAISE EXCEPTION
            'uploads (id, owner_id) are immutable';
    END IF;
    RETURN NEW;
END
$$;
