-- Tier C: enable RLS on events, event_attendees, and uploads. Reuses
-- the Tier-A SD helpers (profiles_in_current_bucket, viewer_profile_ids)
-- since these tables are scoped by profile ownership / stub bucket.
--
-- Shape of the policies (per-command so SELECT visibility doesn't
-- leak into UPDATE / DELETE permission — same lesson as Tier A):
--   * events — same-bucket read, creator-only write
--   * event_attendees — same-bucket read, own-profile write
--   * uploads — public / same-bucket read (anon avatars need to
--     render without auth), owner-only write
--
-- Identity columns on events (id, creator_id) and event_attendees
-- (event_id, profile_id) are pinned via BEFORE UPDATE triggers so a
-- viewer can't retarget their own row into another creator's /
-- event's namespace.

-- ---------------------------------------------------------------------------
-- events
-- ---------------------------------------------------------------------------
ALTER TABLE public.events ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.events FORCE ROW LEVEL SECURITY;

CREATE POLICY events_viewer ON public.events
    FOR SELECT TO poziomki_api
    USING (creator_id IN (SELECT id FROM app.profiles_in_current_bucket()));

CREATE POLICY events_insert ON public.events
    FOR INSERT TO poziomki_api
    WITH CHECK (
        app.current_user_id() > 0
        AND creator_id IN (SELECT id FROM app.viewer_profile_ids())
    );

CREATE POLICY events_update ON public.events
    FOR UPDATE TO poziomki_api
    USING (creator_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (creator_id IN (SELECT id FROM app.viewer_profile_ids()));

CREATE POLICY events_delete ON public.events
    FOR DELETE TO poziomki_api
    USING (creator_id IN (SELECT id FROM app.viewer_profile_ids()));

-- events.creator_id pinned — a viewer can't hand their event to
-- another profile via UPDATE to escape later policy checks, and id
-- must stay stable so attendees + conversations continue resolving.
CREATE OR REPLACE FUNCTION app.reject_events_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.creator_id IS DISTINCT FROM OLD.creator_id THEN
        RAISE EXCEPTION
            'events (id, creator_id) are immutable';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS events_identity_immutable ON public.events;
CREATE TRIGGER events_identity_immutable
    BEFORE UPDATE ON public.events
    FOR EACH ROW
    EXECUTE FUNCTION app.reject_events_identity_change();

-- Event ownership helper — true iff the viewer's profile is the
-- creator of the given event. Used by event_attendees write policies
-- so the creator can approve / reject / delete pending attendees even
-- though the affected row isn't their own attendance.
CREATE OR REPLACE FUNCTION app.viewer_owns_event(p_event_id uuid)
RETURNS boolean
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT app.current_user_id() > 0 AND EXISTS (
        SELECT 1
        FROM public.events e
        JOIN public.profiles p ON p.id = e.creator_id
        WHERE e.id = p_event_id
          AND p.user_id = app.current_user_id()
    )
$$;

COMMENT ON FUNCTION app.viewer_owns_event(uuid) IS
    'True iff the viewer owns the given event. Narrower than viewer_can_access_event (which also admits going attendees) — used for creator-only mutations on the attendee roster.';

REVOKE EXECUTE ON FUNCTION app.viewer_owns_event(uuid) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_owns_event(uuid) TO poziomki_api';
    END IF;
END
$$;

-- Row-level status shape check so a direct API-role caller can't set
-- status to an unknown value; transition semantics (approval,
-- capacity) stay app-layer, but the allowed-status set is constrained
-- at the DB so RLS + this CHECK cover the raw DML surface.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'event_attendees_status_valid'
          AND conrelid = 'public.event_attendees'::regclass
    ) THEN
        ALTER TABLE public.event_attendees
            ADD CONSTRAINT event_attendees_status_valid
            CHECK (status IN ('pending', 'going', 'declined', 'waitlist'));
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- event_attendees
--
-- Reads same-bucket so an attendee roster is answerable for any event
-- in the viewer's bucket. Writes allowed when:
--   * profile_id is one of the viewer's own profiles (self attendance
--     — join, leave, update own status), OR
--   * the viewer owns the event (creator approves / rejects / deletes
--     pending attendees; handler enforces capacity + approval rules).
-- ---------------------------------------------------------------------------
ALTER TABLE public.event_attendees ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.event_attendees FORCE ROW LEVEL SECURITY;

CREATE POLICY event_attendees_viewer ON public.event_attendees
    FOR SELECT TO poziomki_api
    USING (profile_id IN (SELECT id FROM app.profiles_in_current_bucket()));

CREATE POLICY event_attendees_insert ON public.event_attendees
    FOR INSERT TO poziomki_api
    WITH CHECK (
        app.current_user_id() > 0
        AND (
            profile_id IN (SELECT id FROM app.viewer_profile_ids())
            OR app.viewer_owns_event(event_id)
        )
    );

CREATE POLICY event_attendees_update ON public.event_attendees
    FOR UPDATE TO poziomki_api
    USING (
        profile_id IN (SELECT id FROM app.viewer_profile_ids())
        OR app.viewer_owns_event(event_id)
    )
    WITH CHECK (
        profile_id IN (SELECT id FROM app.viewer_profile_ids())
        OR app.viewer_owns_event(event_id)
    );

CREATE POLICY event_attendees_delete ON public.event_attendees
    FOR DELETE TO poziomki_api
    USING (
        profile_id IN (SELECT id FROM app.viewer_profile_ids())
        OR app.viewer_owns_event(event_id)
    );

CREATE OR REPLACE FUNCTION app.reject_event_attendees_pk_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.event_id IS DISTINCT FROM OLD.event_id
       OR NEW.profile_id IS DISTINCT FROM OLD.profile_id THEN
        RAISE EXCEPTION
            'event_attendees primary key is immutable';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS event_attendees_pk_immutable ON public.event_attendees;
CREATE TRIGGER event_attendees_pk_immutable
    BEFORE UPDATE ON public.event_attendees
    FOR EACH ROW
    EXECUTE FUNCTION app.reject_event_attendees_pk_change();

-- ---------------------------------------------------------------------------
-- uploads
--
-- SELECT is looser than events / attendees because anon avatar URLs
-- (profile_picture on a profile that's in the viewer's bucket) need
-- to resolve against uploads. owner_id NULL covers system / anon
-- uploads; owner_id in the viewer's bucket covers real-user avatars.
--
-- Writes are owner-only: INSERT accepts NULL owner (for signup-time
-- avatars where no profile exists yet) or the viewer's profile ids.
-- UPDATE / DELETE require the viewer owns the row.
-- ---------------------------------------------------------------------------
ALTER TABLE public.uploads ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.uploads FORCE ROW LEVEL SECURITY;

CREATE POLICY uploads_viewer ON public.uploads
    FOR SELECT TO poziomki_api
    USING (
        owner_id IS NULL
        OR owner_id IN (SELECT id FROM app.profiles_in_current_bucket())
    );

CREATE POLICY uploads_insert ON public.uploads
    FOR INSERT TO poziomki_api
    WITH CHECK (
        app.current_user_id() > 0
        AND (
            owner_id IS NULL
            OR owner_id IN (SELECT id FROM app.viewer_profile_ids())
        )
    );

CREATE POLICY uploads_update ON public.uploads
    FOR UPDATE TO poziomki_api
    USING (owner_id IN (SELECT id FROM app.viewer_profile_ids()))
    WITH CHECK (owner_id IN (SELECT id FROM app.viewer_profile_ids()));

CREATE POLICY uploads_delete ON public.uploads
    FOR DELETE TO poziomki_api
    USING (owner_id IN (SELECT id FROM app.viewer_profile_ids()));

-- uploads.id + owner_id pinned. Letting owner_id drift via UPDATE
-- would let a viewer adopt anon uploads or reassign to any profile
-- they own; id must stay stable for the S3 key mapping.
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

DROP TRIGGER IF EXISTS uploads_identity_immutable ON public.uploads;
CREATE TRIGGER uploads_identity_immutable
    BEFORE UPDATE ON public.uploads
    FOR EACH ROW
    EXECUTE FUNCTION app.reject_uploads_identity_change();
