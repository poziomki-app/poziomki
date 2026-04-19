-- Tier B: enable RLS on the chat tables (conversations,
-- conversation_members, messages, message_reactions). Access is
-- membership-scoped: a viewer can only see / mutate rows tied to a
-- conversation they're a member of.
--
-- Every handler is already wrapped in `db::with_viewer_tx`, so
-- `app.user_id` is set before any chat query runs.

-- ---------------------------------------------------------------------------
-- Helper: viewer's conversation ids.
--
-- Membership lookup is used by four different tables' SELECT policies
-- and a couple of write policies, so pull it into a SECURITY DEFINER
-- helper. Definer rights matter for two reasons:
--   1. `conversation_members` will have its own policy after this
--      migration, so a non-SD subquery on it would recursively
--      self-filter and only expose the viewer's own row — fine for
--      own-row checks but wrong for "every conversation I'm in".
--   2. Sibling tables (messages, message_reactions) need membership
--      info even though their own SELECT policies haven't bound yet.
--
-- The `current_user_id() > 0` guard blocks anon / unset-GUC callers.
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION app.viewer_conversation_ids()
RETURNS TABLE (conversation_id uuid)
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT cm.conversation_id
    FROM public.conversation_members cm
    WHERE app.current_user_id() > 0
      AND cm.user_id = app.current_user_id()
$$;

COMMENT ON FUNCTION app.viewer_conversation_ids() IS
    'Conversation ids the current viewer is a member of. SECURITY DEFINER so policies on conversation_members / messages / reactions can embed it without tripping sibling RLS. Returns empty for anon.';

-- Helper: "can the viewer see this message?" Used by message_reactions
-- policies. Single-row EXISTS lookup keeps the check cheap and avoids
-- materialising a full list of viewer-visible message ids.
CREATE OR REPLACE FUNCTION app.viewer_can_see_message(p_message_id uuid)
RETURNS boolean
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM public.messages m
        JOIN public.conversation_members cm ON cm.conversation_id = m.conversation_id
        WHERE m.id = p_message_id
          AND app.current_user_id() > 0
          AND cm.user_id = app.current_user_id()
    )
$$;

COMMENT ON FUNCTION app.viewer_can_see_message(uuid) IS
    'True iff the viewer is a member of the conversation the given message belongs to. SECURITY DEFINER so it bypasses the messages + conversation_members policies installed by this migration.';

-- Helper: "can the viewer access this event?" Gate for creating event
-- conversations — without this the conversations_insert policy would
-- accept any `kind='event'` row and let a compromised API-role caller
-- fabricate chat rooms for events they have no right to. Grants access
-- to the event creator and any `going` attendee; `pending`, `waitlist`,
-- `declined`, etc. are rejected so attendance status maps one-to-one
-- with chat access (same gate the HTTP handler uses in chat/mod.rs).
CREATE OR REPLACE FUNCTION app.viewer_can_access_event(p_event_id uuid)
RETURNS boolean
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT app.current_user_id() > 0 AND (
        EXISTS (
            SELECT 1
            FROM public.events e
            JOIN public.profiles p ON p.id = e.creator_id
            WHERE e.id = p_event_id
              AND p.user_id = app.current_user_id()
        )
        OR EXISTS (
            SELECT 1
            FROM public.event_attendees ea
            JOIN public.profiles p ON p.id = ea.profile_id
            WHERE ea.event_id = p_event_id
              AND ea.status = 'going'
              AND p.user_id = app.current_user_id()
        )
    )
$$;

COMMENT ON FUNCTION app.viewer_can_access_event(uuid) IS
    'True iff the viewer owns the event or has a `going` attendance row. Used by conversations_insert to prevent arbitrary event chat creation; mirrors the HTTP-layer gate.';

-- Helper: user_id behind an event's creator. The event-chat bootstrap
-- (resolve_or_create_event_conversation) inserts the creator membership
-- before the viewer's own, so the conversation_members INSERT policy
-- needs to recognise that specific target user_id as legitimate — not
-- just the viewer's own id.
CREATE OR REPLACE FUNCTION app.event_creator_user_id(p_event_id uuid)
RETURNS int
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT p.user_id
    FROM public.events e
    JOIN public.profiles p ON p.id = e.creator_id
    WHERE e.id = p_event_id
$$;

COMMENT ON FUNCTION app.event_creator_user_id(uuid) IS
    'User id of the given event''s creator, or NULL if missing. SECURITY DEFINER so policy expressions can resolve it without tripping events / profiles RLS.';

-- Helpers for the chat resolve-or-create paths (resolve_or_create_dm
-- and resolve_or_create_event_conversation). They look up an existing
-- conversation row by pair / event regardless of viewer membership so
-- the caller can discover and reuse a row that was created by a
-- concurrent request that hasn't yet added the viewer as member.
-- Without this, the handler's race-fallback SELECT would be filtered
-- to empty by conversations_viewer and bubble up as NotFound.
CREATE OR REPLACE FUNCTION app.find_dm_conversation(p_low int, p_high int)
RETURNS SETOF public.conversations
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT * FROM public.conversations
    WHERE kind = 'dm' AND user_low_id = p_low AND user_high_id = p_high
    LIMIT 1
$$;

COMMENT ON FUNCTION app.find_dm_conversation(int, int) IS
    'Canonical DM conversation row for the (low, high) user pair, or empty set. SECURITY DEFINER so the chat resolve path can read the row before the viewer''s membership has been inserted.';

CREATE OR REPLACE FUNCTION app.find_event_conversation(p_event_id uuid)
RETURNS SETOF public.conversations
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT * FROM public.conversations
    WHERE kind = 'event' AND event_id = p_event_id
    LIMIT 1
$$;

COMMENT ON FUNCTION app.find_event_conversation(uuid) IS
    'Event chat conversation row for the given event, or empty set. SECURITY DEFINER so the resolve-or-create path can detect concurrent creation without relying on viewer membership.';

REVOKE EXECUTE ON FUNCTION app.viewer_conversation_ids() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.viewer_can_see_message(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.viewer_can_access_event(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.event_creator_user_id(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.find_dm_conversation(int, int) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.find_event_conversation(uuid) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_conversation_ids() TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_can_see_message(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_can_access_event(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.event_creator_user_id(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.find_dm_conversation(int, int) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.find_event_conversation(uuid) TO poziomki_api';
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- conversations
--
-- Reads limited to viewer's conversations. Writes scoped tight enough
-- to cover the two legitimate creation flows (DM bootstrap + event chat
-- resolve) without letting a compromised API-role caller fabricate
-- arbitrary rows. UPDATE / DELETE remain membership-gated.
-- ---------------------------------------------------------------------------
ALTER TABLE public.conversations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.conversations FORCE ROW LEVEL SECURITY;

CREATE POLICY conversations_viewer ON public.conversations
    FOR SELECT TO poziomki_api
    USING (id IN (SELECT conversation_id FROM app.viewer_conversation_ids()));

-- DM creation requires the viewer to be one of the pair; event chat
-- creation requires the viewer to own or attend the referenced event.
-- Relying on the handler for event access was insufficient — a
-- compromised API-role caller could fabricate chat rooms for arbitrary
-- events and then add themselves via conversation_members_insert.
CREATE POLICY conversations_insert ON public.conversations
    FOR INSERT TO poziomki_api
    WITH CHECK (
        app.current_user_id() > 0
        AND (
            (kind = 'dm'
             AND app.current_user_id() IN (user_low_id, user_high_id))
            OR (kind = 'event'
                AND event_id IS NOT NULL
                AND app.viewer_can_access_event(event_id))
        )
    );

CREATE POLICY conversations_update ON public.conversations
    FOR UPDATE TO poziomki_api
    USING (id IN (SELECT conversation_id FROM app.viewer_conversation_ids()))
    WITH CHECK (id IN (SELECT conversation_id FROM app.viewer_conversation_ids()));

CREATE POLICY conversations_delete ON public.conversations
    FOR DELETE TO poziomki_api
    USING (id IN (SELECT conversation_id FROM app.viewer_conversation_ids()));

-- ---------------------------------------------------------------------------
-- conversation_members
--
-- Reads cover: own rows + rows in conversations the viewer is in
-- (so "who else is in this DM / event chat" is answerable). Writes
-- are own-row-only for UPDATE/DELETE and INSERT must name a valid
-- bootstrap shape:
--   * DM — viewer is one of the pair AND the inserted user_id is
--     also one of the pair (blocks third-party injection).
--   * Event — the conversation is a legitimate event chat (creator
--     or attendee can resolve), and the inserted user_id is either
--     the viewer themselves or the event's creator (the chat
--     bootstrap inserts creator first, then attendee self-join).
--
-- A BEFORE UPDATE trigger pins (conversation_id, user_id) — they're
-- the primary key and must never move — because RLS predicates can't
-- compare old-row vs new-row values, only the final row. Without the
-- trigger a viewer could `UPDATE ... SET conversation_id = <target>`
-- and migrate their membership row into a conversation they aren't in.
-- ---------------------------------------------------------------------------
ALTER TABLE public.conversation_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.conversation_members FORCE ROW LEVEL SECURITY;

CREATE POLICY conversation_members_viewer ON public.conversation_members
    FOR SELECT TO poziomki_api
    USING (
        user_id = app.current_user_id()
        OR conversation_id IN (SELECT conversation_id FROM app.viewer_conversation_ids())
    );

CREATE POLICY conversation_members_insert ON public.conversation_members
    FOR INSERT TO poziomki_api
    WITH CHECK (
        app.current_user_id() > 0
        AND EXISTS (
            SELECT 1
            FROM public.conversations c
            WHERE c.id = conversation_id
              AND (
                  (c.kind = 'dm'
                   AND app.current_user_id() IN (c.user_low_id, c.user_high_id)
                   AND conversation_members.user_id IN (c.user_low_id, c.user_high_id))
                  OR (c.kind = 'event'
                      AND c.event_id IS NOT NULL
                      AND app.viewer_can_access_event(c.event_id)
                      AND (
                          conversation_members.user_id = app.current_user_id()
                          OR conversation_members.user_id = app.event_creator_user_id(c.event_id)
                      ))
              )
        )
    );

CREATE POLICY conversation_members_update ON public.conversation_members
    FOR UPDATE TO poziomki_api
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());

CREATE POLICY conversation_members_delete ON public.conversation_members
    FOR DELETE TO poziomki_api
    USING (user_id = app.current_user_id());

CREATE OR REPLACE FUNCTION app.reject_conversation_members_pk_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.conversation_id IS DISTINCT FROM OLD.conversation_id
       OR NEW.user_id IS DISTINCT FROM OLD.user_id THEN
        RAISE EXCEPTION
            'conversation_members primary key is immutable (old=%,% new=%,%)',
            OLD.conversation_id, OLD.user_id, NEW.conversation_id, NEW.user_id;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS conversation_members_pk_immutable ON public.conversation_members;
CREATE TRIGGER conversation_members_pk_immutable
    BEFORE UPDATE ON public.conversation_members
    FOR EACH ROW
    EXECUTE FUNCTION app.reject_conversation_members_pk_change();

-- ---------------------------------------------------------------------------
-- messages
--
-- Reads limited to conversations the viewer belongs to. Writes are
-- sender-only: only the viewer can send/edit/delete as themselves,
-- and the INSERT WITH CHECK additionally requires the target
-- conversation is one they're in (no blind cross-conversation
-- message injection).
-- ---------------------------------------------------------------------------
ALTER TABLE public.messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.messages FORCE ROW LEVEL SECURITY;

CREATE POLICY messages_viewer ON public.messages
    FOR SELECT TO poziomki_api
    USING (conversation_id IN (SELECT conversation_id FROM app.viewer_conversation_ids()));

CREATE POLICY messages_insert ON public.messages
    FOR INSERT TO poziomki_api
    WITH CHECK (
        sender_id = app.current_user_id()
        AND conversation_id IN (SELECT conversation_id FROM app.viewer_conversation_ids())
    );

-- UPDATE / DELETE require current membership in addition to authorship,
-- so a user who leaves a conversation loses edit rights to their own
-- past messages there. Tightening to membership matches the invariant
-- "writes are gated by current access, not historical authorship".
CREATE POLICY messages_update ON public.messages
    FOR UPDATE TO poziomki_api
    USING (
        sender_id = app.current_user_id()
        AND conversation_id IN (SELECT conversation_id FROM app.viewer_conversation_ids())
    )
    WITH CHECK (
        sender_id = app.current_user_id()
        AND conversation_id IN (SELECT conversation_id FROM app.viewer_conversation_ids())
    );

CREATE POLICY messages_delete ON public.messages
    FOR DELETE TO poziomki_api
    USING (
        sender_id = app.current_user_id()
        AND conversation_id IN (SELECT conversation_id FROM app.viewer_conversation_ids())
    );

-- ---------------------------------------------------------------------------
-- message_reactions
--
-- Reads follow message visibility (you can see a reaction iff you can
-- see the message). Writes are user-scoped: only your own reaction
-- rows, and only on messages you can see.
-- ---------------------------------------------------------------------------
ALTER TABLE public.message_reactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.message_reactions FORCE ROW LEVEL SECURITY;

CREATE POLICY message_reactions_viewer ON public.message_reactions
    FOR SELECT TO poziomki_api
    USING (app.viewer_can_see_message(message_id));

CREATE POLICY message_reactions_insert ON public.message_reactions
    FOR INSERT TO poziomki_api
    WITH CHECK (
        user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    );

-- UPDATE / DELETE keep the user scoping AND require the viewer still
-- sees the parent message. Without the viewer_can_see_message check a
-- viewer who knows a message UUID could mutate their own reaction onto
-- messages outside their conversations.
CREATE POLICY message_reactions_update ON public.message_reactions
    FOR UPDATE TO poziomki_api
    USING (
        user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    )
    WITH CHECK (
        user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    );

CREATE POLICY message_reactions_delete ON public.message_reactions
    FOR DELETE TO poziomki_api
    USING (
        user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    );
