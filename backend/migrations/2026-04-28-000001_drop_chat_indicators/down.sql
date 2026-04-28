-- Chat indicators: per-message read receipts, server-confirmed delivery
-- ACK, and per-conversation mute. Brings the chat closer to behavior
-- users expect from real messaging apps:
--
--  * `message_reads` records WHO read WHICH message and WHEN, so the
--    sender can render single/double/double-blue ticks and group chats
--    can show a "read by" list. The existing
--    `conversation_members.last_read_message_id` watermark stays as
--    the fast-path for unread-count math; this table is the truth for
--    UI ticks and survives reconnect (history hydration).
--
--  * `message_deliveries` records when a recipient's WS session has
--    actually received a message. Today the mobile client guesses
--    "sent" the moment it POSTs; with this table the server can emit
--    a real Delivered ACK so the sender sees ✓ → ✓✓ accurately, even
--    across reconnects.
--
--  * `conversation_members.muted_until` lets a user silence one DM or
--    group without disabling all notifications. NULL = not muted.
--    A far-future timestamp encodes "muted forever".

CREATE TABLE IF NOT EXISTS public.message_reads (
    message_id UUID NOT NULL REFERENCES public.messages(id) ON DELETE CASCADE,
    user_id    INT4 NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    read_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_message_reads_user
    ON public.message_reads (user_id, read_at DESC);

COMMENT ON TABLE public.message_reads IS
    'Per-(message, user) read receipts with timestamp. Powers ✓✓ ticks and "read by" UI; survives reconnect via history hydration.';

CREATE TABLE IF NOT EXISTS public.message_deliveries (
    message_id   UUID NOT NULL REFERENCES public.messages(id) ON DELETE CASCADE,
    user_id      INT4 NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    delivered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_message_deliveries_user
    ON public.message_deliveries (user_id, delivered_at DESC);

COMMENT ON TABLE public.message_deliveries IS
    'Per-(message, recipient) delivery confirmations written when the WS session actually receives the message. Powers single→double tick transition.';

ALTER TABLE public.conversation_members
    ADD COLUMN IF NOT EXISTS muted_until TIMESTAMPTZ NULL;

COMMENT ON COLUMN public.conversation_members.muted_until IS
    'Per-conversation mute. NULL = not muted; future timestamp = muted until that moment; far-future = "muted forever".';

-- RLS: mirror messages — a member of the conversation can read their
-- own and other members' read/delivery rows for that conversation.
-- Inserts are gated to the current user (you can only mark your own
-- read/delivery; the API sets app.current_user_id correctly).

ALTER TABLE public.message_reads ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.message_reads FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS message_reads_select ON public.message_reads;
CREATE POLICY message_reads_select ON public.message_reads
    FOR SELECT
    USING (app.viewer_can_see_message(message_id));

DROP POLICY IF EXISTS message_reads_insert ON public.message_reads;
CREATE POLICY message_reads_insert ON public.message_reads
    FOR INSERT
    WITH CHECK (
        user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    );

ALTER TABLE public.message_deliveries ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.message_deliveries FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS message_deliveries_select ON public.message_deliveries;
CREATE POLICY message_deliveries_select ON public.message_deliveries
    FOR SELECT
    USING (app.viewer_can_see_message(message_id));

DROP POLICY IF EXISTS message_deliveries_insert ON public.message_deliveries;
CREATE POLICY message_deliveries_insert ON public.message_deliveries
    FOR INSERT
    WITH CHECK (
        user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    );

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT INSERT, SELECT ON public.message_reads TO poziomki_api';
        EXECUTE 'GRANT INSERT, SELECT ON public.message_deliveries TO poziomki_api';
    END IF;
END
$$;

-- Server-confirmed delivery is written by the API the moment the
-- WS hub fans out a Message to a recipient — there is no per-user
-- "I received it" call from the client, so we cannot rely on
-- `app.current_user_id() = user_id` for the insert. The SECURITY
-- DEFINER helper checks membership instead. The helper is the only
-- way to insert into `message_deliveries` for a user other than
-- `app.current_user_id()`.
CREATE OR REPLACE FUNCTION app.record_delivery(p_message_id uuid, p_user_id int)
RETURNS timestamptz
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    v_conv_id uuid;
    v_at timestamptz;
BEGIN
    SELECT conversation_id INTO v_conv_id
    FROM public.messages
    WHERE id = p_message_id;
    IF v_conv_id IS NULL THEN
        RETURN NULL;
    END IF;

    -- Recipient must actually be a member of the conversation; the
    -- WS handler computes that, but defense-in-depth here.
    IF NOT EXISTS (
        SELECT 1 FROM public.conversation_members
        WHERE conversation_id = v_conv_id AND user_id = p_user_id
    ) THEN
        RETURN NULL;
    END IF;

    INSERT INTO public.message_deliveries (message_id, user_id, delivered_at)
    VALUES (p_message_id, p_user_id, now())
    ON CONFLICT (message_id, user_id) DO NOTHING
    RETURNING delivered_at INTO v_at;

    RETURN v_at;
END
$$;

REVOKE EXECUTE ON FUNCTION app.record_delivery(uuid, int) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.record_delivery(uuid, int) TO poziomki_api';
    END IF;
END
$$;

-- Bulk variant. The WS hub fans out to N online recipients per send;
-- looping `record_delivery` once per recipient meant N round trips per
-- broadcast (one short tx each). For groups this becomes the dominant
-- cost. This variant inserts every (message_id, user_id) row in a
-- single statement and returns the (user_id, delivered_at) pairs that
-- were actually inserted, so the caller still knows which sender
-- broadcasts to emit. Membership is verified in the same statement.
CREATE OR REPLACE FUNCTION app.record_deliveries(p_message_id uuid, p_user_ids int[])
RETURNS TABLE (user_id int, delivered_at timestamptz)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public, pg_temp
AS $$
DECLARE
    v_conv_id uuid;
BEGIN
    SELECT conversation_id INTO v_conv_id
    FROM public.messages
    WHERE id = p_message_id;
    IF v_conv_id IS NULL OR p_user_ids IS NULL OR array_length(p_user_ids, 1) IS NULL THEN
        RETURN;
    END IF;

    RETURN QUERY
    INSERT INTO public.message_deliveries (message_id, user_id, delivered_at)
    SELECT p_message_id, cm.user_id, now()
    FROM public.conversation_members cm
    WHERE cm.conversation_id = v_conv_id
      AND cm.user_id = ANY (p_user_ids)
    ON CONFLICT (message_id, user_id) DO NOTHING
    RETURNING message_deliveries.user_id, message_deliveries.delivered_at;
END
$$;

REVOKE EXECUTE ON FUNCTION app.record_deliveries(uuid, int[]) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.record_deliveries(uuid, int[]) TO poziomki_api';
    END IF;
END
$$;

-- Backfill: synthesize read receipts from the existing watermarks so
-- old conversations don't render with blank ticks after the upgrade.
-- For each (member, conversation) pair, mark every non-self message
-- created at or before the member's last_read_message_id as read by
-- that member. read_at is best-effort (use the message's created_at
-- as a lower bound — they read it at least that late).
INSERT INTO public.message_reads (message_id, user_id, read_at)
SELECT m.id, cm.user_id, m.created_at
FROM public.conversation_members cm
JOIN public.messages rm ON rm.id = cm.last_read_message_id
JOIN public.messages m
  ON m.conversation_id = cm.conversation_id
 AND m.sender_id <> cm.user_id
 AND m.deleted_at IS NULL
 AND (m.created_at < rm.created_at
      OR (m.created_at = rm.created_at AND m.id <= rm.id))
ON CONFLICT (message_id, user_id) DO NOTHING;

-- Backfill deliveries for the same (member, message) universe: every
-- non-self, non-deleted message in a conversation the user belongs to.
-- Without this, every existing chat regresses to a single grey ✓ on
-- the sender's screen on first launch after the upgrade — even though
-- the message was clearly delivered (the recipient already read some
-- of them). We use the message's created_at as the delivered_at floor;
-- in practice delivery happened within seconds of send, so anchoring
-- at created_at is a safe lower bound that won't lie about ordering.
INSERT INTO public.message_deliveries (message_id, user_id, delivered_at)
SELECT m.id, cm.user_id, m.created_at
FROM public.conversation_members cm
JOIN public.messages m
  ON m.conversation_id = cm.conversation_id
 AND m.sender_id <> cm.user_id
 AND m.deleted_at IS NULL
ON CONFLICT (message_id, user_id) DO NOTHING;
