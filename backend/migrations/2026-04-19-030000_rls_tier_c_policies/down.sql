DROP TRIGGER IF EXISTS uploads_identity_immutable ON public.uploads;
DROP FUNCTION IF EXISTS app.reject_uploads_identity_change();
DROP POLICY IF EXISTS uploads_delete ON public.uploads;
DROP POLICY IF EXISTS uploads_update ON public.uploads;
DROP POLICY IF EXISTS uploads_insert ON public.uploads;
DROP POLICY IF EXISTS uploads_viewer ON public.uploads;

DROP TRIGGER IF EXISTS event_attendees_pk_immutable ON public.event_attendees;
DROP FUNCTION IF EXISTS app.reject_event_attendees_pk_change();
DROP POLICY IF EXISTS event_attendees_delete ON public.event_attendees;
DROP POLICY IF EXISTS event_attendees_update ON public.event_attendees;
DROP POLICY IF EXISTS event_attendees_insert ON public.event_attendees;
DROP POLICY IF EXISTS event_attendees_viewer ON public.event_attendees;

DROP TRIGGER IF EXISTS events_identity_immutable ON public.events;
DROP FUNCTION IF EXISTS app.reject_events_identity_change();
DROP POLICY IF EXISTS events_delete ON public.events;
DROP POLICY IF EXISTS events_update ON public.events;
DROP POLICY IF EXISTS events_insert ON public.events;
DROP POLICY IF EXISTS events_viewer ON public.events;

ALTER TABLE public.uploads NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.uploads DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.event_attendees NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.event_attendees DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.events NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.events DISABLE ROW LEVEL SECURITY;
