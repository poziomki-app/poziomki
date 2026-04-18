-- Narrow public-projection helpers used by the events module.
--
-- Attendee listing and the event-approval push notification need a few
-- small facts from `users` and `profiles` that would otherwise require
-- cross-user SELECT on rows containing sensitive columns. These SD
-- helpers expose exactly what the caller needs and no more, so the API
-- role can stay at least-privilege when Tier-A RLS lands.
--
-- All `app.*` SECURITY DEFINER functions use `SET search_path = pg_catalog,
-- pg_temp` and schema-qualified table names to defeat pg_temp search-path
-- hijacks. The `sd_helpers_harden_search_path` migration applies the same
-- header to the prior 010000..040000 helpers in one place.

-- Batch lookup of (id, pid) pairs for a list of user ids. Used by the
-- attendee listing endpoint to resolve user pids without granting the
-- API role a broad SELECT on users.
CREATE OR REPLACE FUNCTION app.user_pids_for_ids(p_user_ids int[])
RETURNS TABLE (user_id int, pid uuid)
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT id, pid FROM public.users WHERE id = ANY(p_user_ids)
$$;

COMMENT ON FUNCTION app.user_pids_for_ids(int[]) IS
    'Batch lookup of (user_id, pid) for a list of users. Narrow projection; used by event attendee listing.';

-- Resolve the owner user_id for a profile. Used by the event-approval
-- push notification path where the handler has a profile_id and needs
-- the owning user to dispatch a push. Narrow projection: no profile
-- fields leak, only the owning user_id.
CREATE OR REPLACE FUNCTION app.profile_owner_user_id(p_profile_id uuid)
RETURNS int
LANGUAGE sql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
STABLE
AS $$
    SELECT user_id FROM public.profiles WHERE id = p_profile_id
$$;

COMMENT ON FUNCTION app.profile_owner_user_id(uuid) IS
    'Resolve the owner user_id of a profile. Narrow projection; used by event approval push notifications.';

REVOKE EXECUTE ON FUNCTION app.user_pids_for_ids(int[]) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.profile_owner_user_id(uuid) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.user_pids_for_ids(int[]) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.profile_owner_user_id(uuid) TO poziomki_api';
    END IF;
END
$$;
