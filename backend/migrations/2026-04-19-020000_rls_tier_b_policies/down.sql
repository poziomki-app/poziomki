DROP TRIGGER IF EXISTS message_reactions_key_immutable ON public.message_reactions;
DROP FUNCTION IF EXISTS app.reject_message_reactions_key_change();
DROP POLICY IF EXISTS message_reactions_delete ON public.message_reactions;
DROP POLICY IF EXISTS message_reactions_update ON public.message_reactions;
DROP POLICY IF EXISTS message_reactions_insert ON public.message_reactions;
DROP POLICY IF EXISTS message_reactions_viewer ON public.message_reactions;

DROP TRIGGER IF EXISTS messages_conversation_immutable ON public.messages;
DROP FUNCTION IF EXISTS app.reject_messages_conversation_change();
DROP POLICY IF EXISTS messages_delete ON public.messages;
DROP POLICY IF EXISTS messages_update ON public.messages;
DROP POLICY IF EXISTS messages_insert ON public.messages;
DROP POLICY IF EXISTS messages_viewer ON public.messages;

DROP TRIGGER IF EXISTS conversation_members_pk_immutable ON public.conversation_members;
DROP FUNCTION IF EXISTS app.reject_conversation_members_pk_change();
DROP POLICY IF EXISTS conversation_members_delete ON public.conversation_members;
DROP POLICY IF EXISTS conversation_members_update ON public.conversation_members;
DROP POLICY IF EXISTS conversation_members_insert ON public.conversation_members;
DROP POLICY IF EXISTS conversation_members_viewer ON public.conversation_members;

DROP TRIGGER IF EXISTS conversations_identity_immutable ON public.conversations;
DROP FUNCTION IF EXISTS app.reject_conversations_identity_change();
DROP POLICY IF EXISTS conversations_delete ON public.conversations;
DROP POLICY IF EXISTS conversations_update ON public.conversations;
DROP POLICY IF EXISTS conversations_insert ON public.conversations;
DROP POLICY IF EXISTS conversations_viewer ON public.conversations;

ALTER TABLE public.message_reactions NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.message_reactions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.messages NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.messages DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.conversation_members NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.conversation_members DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.conversations NO FORCE ROW LEVEL SECURITY;
ALTER TABLE public.conversations DISABLE ROW LEVEL SECURITY;

DROP FUNCTION IF EXISTS app.conversation_meta_for_insert(uuid);
DROP FUNCTION IF EXISTS app.find_event_conversation(uuid);
DROP FUNCTION IF EXISTS app.find_dm_conversation(int, int);
DROP FUNCTION IF EXISTS app.session_bypasses_rls();
DROP FUNCTION IF EXISTS app.event_creator_user_id(uuid);
DROP FUNCTION IF EXISTS app.viewer_can_access_event(uuid);
DROP FUNCTION IF EXISTS app.viewer_can_see_message(uuid);
DROP FUNCTION IF EXISTS app.viewer_conversation_ids();
