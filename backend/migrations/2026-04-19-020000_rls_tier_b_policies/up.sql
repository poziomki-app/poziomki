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
-- Helper: does the session-level caller already have BYPASSRLS? The
-- worker process connects as `poziomki_worker` which has BYPASSRLS on
-- real tables, but inside a SECURITY DEFINER function `current_user`
-- resolves to the definer (owner), so the SD lookup helpers need an
-- explicit out for the worker. `session_user` preserves the role the
-- connection authenticated as. Future BYPASSRLS roles inherit this
-- without a code change.
CREATE OR REPLACE FUNCTION app.session_bypasses_rls()
RETURNS boolean
LANGUAGE sql
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT COALESCE(
        (SELECT rolbypassrls FROM pg_catalog.pg_roles WHERE rolname = session_user),
        false
    )
$$;

COMMENT ON FUNCTION app.session_bypasses_rls() IS
    'True iff the session authentication role has BYPASSRLS. Used inside SD helpers to give worker processes a trust bypass that the policy guards would otherwise block.';

-- DM lookup gates on the viewer being one of the pair — without that
-- guard the SD bypass would let any API-role caller enumerate DM
-- conversation ids by iterating over (low, high) pairs. Worker
-- sessions (poziomki_worker / BYPASSRLS) are trusted and skip the
-- check so background jobs like event membership sync still work.
CREATE OR REPLACE FUNCTION app.find_dm_conversation(p_low int, p_high int)
RETURNS SETOF public.conversations
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT * FROM public.conversations
    WHERE kind = 'dm'
      AND user_low_id = p_low
      AND user_high_id = p_high
      AND (
          app.session_bypasses_rls()
          OR (app.current_user_id() > 0
              AND app.current_user_id() IN (p_low, p_high))
      )
    LIMIT 1
$$;

COMMENT ON FUNCTION app.find_dm_conversation(int, int) IS
    'Canonical DM conversation row for the (low, high) user pair, restricted to viewers who are one of the pair. SECURITY DEFINER so it works before the viewer''s membership row has been inserted.';

-- Event lookup gates on viewer event access (creator or going
-- attendee) via the existing helper, so the SD bypass can't be used
-- to discover the conversation id of an event the viewer has no
-- relationship to.
CREATE OR REPLACE FUNCTION app.find_event_conversation(p_event_id uuid)
RETURNS SETOF public.conversations
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT * FROM public.conversations
    WHERE kind = 'event'
      AND event_id = p_event_id
      AND (
          app.session_bypasses_rls()
          OR app.viewer_can_access_event(p_event_id)
      )
    LIMIT 1
$$;

COMMENT ON FUNCTION app.find_event_conversation(uuid) IS
    'Event chat conversation row for the given event, restricted to viewers with event access (owner or going attendee). SECURITY DEFINER so the resolve-or-create path can detect concurrent creation before the viewer''s membership row exists.';

-- conversation_members_insert has to look up the target conversation
-- to decide if the proposed (conversation_id, user_id) pair is
-- legitimate. A plain subquery on public.conversations would be
-- filtered by conversations_viewer — fine for viewers already in the
-- room, but the resolve-or-create event-chat bootstrap reaches the
-- INSERT before the viewer's membership row exists. This SD helper
-- returns just the fields the INSERT policy needs, without membership
-- filtering, so going attendees can self-join.
CREATE OR REPLACE FUNCTION app.conversation_meta_for_insert(p_conversation_id uuid)
RETURNS TABLE (kind text, event_id uuid, user_low_id int, user_high_id int)
LANGUAGE sql
SECURITY DEFINER
STABLE
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT c.kind, c.event_id, c.user_low_id, c.user_high_id
    FROM public.conversations c
    WHERE app.current_user_id() > 0
      AND c.id = p_conversation_id
      AND (
          (c.kind = 'dm'
           AND app.current_user_id() IN (c.user_low_id, c.user_high_id))
          OR (c.kind = 'event'
              AND c.event_id IS NOT NULL
              AND app.viewer_can_access_event(c.event_id))
      )
$$;

COMMENT ON FUNCTION app.conversation_meta_for_insert(uuid) IS
    'Metadata the conversation_members INSERT policy needs to decide if a (conversation_id, user_id) pair is legitimate. SECURITY DEFINER so it doesn''t self-filter through conversations_viewer.';

REVOKE EXECUTE ON FUNCTION app.viewer_conversation_ids() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.viewer_can_see_message(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.viewer_can_access_event(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.event_creator_user_id(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.session_bypasses_rls() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.find_dm_conversation(int, int) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.find_event_conversation(uuid) FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.conversation_meta_for_insert(uuid) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.session_bypasses_rls() TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_conversation_ids() TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_can_see_message(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_can_access_event(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.event_creator_user_id(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.find_dm_conversation(int, int) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.find_event_conversation(uuid) TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.conversation_meta_for_insert(uuid) TO poziomki_api';
    END IF;
    -- Worker process (BYPASSRLS) reaches the resolve-or-create event
    -- chat path via sync_event_membership — it needs execute on the
    -- lookup helpers even though its row-level bypass means the
    -- INSERT policies are irrelevant to it.
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_worker') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA app TO poziomki_worker';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.session_bypasses_rls() TO poziomki_worker';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.find_dm_conversation(int, int) TO poziomki_worker';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.find_event_conversation(uuid) TO poziomki_worker';
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

-- conversations.(kind, event_id, user_low_id, user_high_id) are
-- identity columns — the policy branches below decide what an
-- INSERT is allowed to be, but a subsequent UPDATE could mutate
-- those fields and sidestep the intent. Concrete attacks:
--   * flip kind='dm' → 'event' to switch which policy branch
--     conversation_members_insert applies
--   * rewrite user_low/high on a DM to silently enrol a third party
--   * rewrite event_id to attach the chat to a different event
-- RLS can't compare old-row vs new-row values, so a BEFORE UPDATE
-- trigger pins them; title / timestamp columns remain mutable.
CREATE OR REPLACE FUNCTION app.reject_conversations_identity_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.kind IS DISTINCT FROM OLD.kind
       OR NEW.event_id IS DISTINCT FROM OLD.event_id
       OR NEW.user_low_id IS DISTINCT FROM OLD.user_low_id
       OR NEW.user_high_id IS DISTINCT FROM OLD.user_high_id THEN
        RAISE EXCEPTION
            'conversations identity columns (kind, event_id, user_low_id, user_high_id) are immutable';
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS conversations_identity_immutable ON public.conversations;
CREATE TRIGGER conversations_identity_immutable
    BEFORE UPDATE ON public.conversations
    FOR EACH ROW
    EXECUTE FUNCTION app.reject_conversations_identity_change();

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
            FROM app.conversation_meta_for_insert(conversation_id) c
            WHERE (
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

-- messages.conversation_id + sender_id are immutable. RLS can't
-- compare old-row vs new-row values, so without this trigger a
-- member of two conversations could `UPDATE messages SET
-- conversation_id = <other>` and inject their message into the
-- second room — both USING (old conversation membership) and WITH
-- CHECK (new conversation membership) would pass.
CREATE OR REPLACE FUNCTION app.reject_messages_conversation_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.conversation_id IS DISTINCT FROM OLD.conversation_id
       OR NEW.sender_id IS DISTINCT FROM OLD.sender_id THEN
        RAISE EXCEPTION
            'messages (conversation_id, sender_id) are immutable (old=%,% new=%,%)',
            OLD.conversation_id, OLD.sender_id, NEW.conversation_id, NEW.sender_id;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS messages_conversation_immutable ON public.messages;
CREATE TRIGGER messages_conversation_immutable
    BEFORE UPDATE ON public.messages
    FOR EACH ROW
    EXECUTE FUNCTION app.reject_messages_conversation_change();

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

-- message_reactions.message_id + user_id are immutable for the same
-- reason as messages and conversation_members: RLS evaluates USING
-- against the old row and WITH CHECK against the new row
-- independently, so a viewer who can see two different messages can
-- move their reaction between them (cross-conversation reaction
-- injection) without either predicate catching it.
CREATE OR REPLACE FUNCTION app.reject_message_reactions_key_change()
RETURNS trigger
LANGUAGE plpgsql
SET search_path = pg_catalog, pg_temp
AS $$
BEGIN
    IF NEW.message_id IS DISTINCT FROM OLD.message_id
       OR NEW.user_id IS DISTINCT FROM OLD.user_id THEN
        RAISE EXCEPTION
            'message_reactions (message_id, user_id) are immutable (old=%,% new=%,%)',
            OLD.message_id, OLD.user_id, NEW.message_id, NEW.user_id;
    END IF;
    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS message_reactions_key_immutable ON public.message_reactions;
CREATE TRIGGER message_reactions_key_immutable
    BEFORE UPDATE ON public.message_reactions
    FOR EACH ROW
    EXECUTE FUNCTION app.reject_message_reactions_key_change();
