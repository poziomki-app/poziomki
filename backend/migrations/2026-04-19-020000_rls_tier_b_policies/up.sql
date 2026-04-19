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

REVOKE EXECUTE ON FUNCTION app.viewer_conversation_ids() FROM PUBLIC;
REVOKE EXECUTE ON FUNCTION app.viewer_can_see_message(uuid) FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_conversation_ids() TO poziomki_api';
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.viewer_can_see_message(uuid) TO poziomki_api';
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

-- DM creation writes a row with `kind='dm'` and the viewer as one of
-- the pair; event chat creation writes a row with `kind='event'` on
-- behalf of an authenticated user (handler verifies event access).
-- The anon guard (`current_user_id > 0`) blocks unset-GUC callers
-- from creating rows.
CREATE POLICY conversations_insert ON public.conversations
    FOR INSERT TO poziomki_api
    WITH CHECK (
        app.current_user_id() > 0
        AND (
            (kind = 'dm'
             AND app.current_user_id() IN (user_low_id, user_high_id))
            OR kind = 'event'
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
-- are own-row-only for UPDATE/DELETE (viewer bumps their own
-- last_read_message_id, viewer leaves a chat). INSERT supports the
-- two legitimate bootstraps: adding yourself, or adding the DM
-- counterpart when you're one of the pair.
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
        AND (
            user_id = app.current_user_id()
            OR EXISTS (
                SELECT 1
                FROM public.conversations c
                WHERE c.id = conversation_id
                  AND c.kind = 'dm'
                  AND app.current_user_id() IN (c.user_low_id, c.user_high_id)
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

CREATE POLICY messages_update ON public.messages
    FOR UPDATE TO poziomki_api
    USING (sender_id = app.current_user_id())
    WITH CHECK (sender_id = app.current_user_id());

CREATE POLICY messages_delete ON public.messages
    FOR DELETE TO poziomki_api
    USING (sender_id = app.current_user_id());

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

CREATE POLICY message_reactions_update ON public.message_reactions
    FOR UPDATE TO poziomki_api
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());

CREATE POLICY message_reactions_delete ON public.message_reactions
    FOR DELETE TO poziomki_api
    USING (user_id = app.current_user_id());
