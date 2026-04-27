-- Chat-message moderation blur + reveal audit.
--
-- Today the moderation_scan outbox dispatcher only logs a verdict. To
-- let clients render flagged messages with a tap-to-reveal blur, the
-- verdict has to be persisted on the row and broadcast over WS.
--
-- `moderation_verdict` is NULL until the async scan completes (clients
-- treat NULL as "allow" so unscanned messages render normally during
-- the scan window). After scanning, it's one of `allow` / `flag` /
-- `block`. We don't enforce a CHECK constraint to keep adding new
-- verdicts cheap; the worker writes a closed set.
--
-- `moderation_categories` carries the flagged subset (e.g.
-- {"vulgar","hate"}) so the client can render a category-aware
-- explanation without round-tripping. Empty array when verdict=allow.

ALTER TABLE public.messages
    ADD COLUMN IF NOT EXISTS moderation_verdict TEXT,
    ADD COLUMN IF NOT EXISTS moderation_categories TEXT[] NOT NULL DEFAULT '{}'::text[],
    ADD COLUMN IF NOT EXISTS moderation_scanned_at TIMESTAMPTZ;

COMMENT ON COLUMN public.messages.moderation_verdict IS
    'Bielik-Guard verdict: NULL=not yet scanned, ''allow''/''flag''/''block''. Clients render NULL as allow.';
COMMENT ON COLUMN public.messages.moderation_categories IS
    'Categories above their flag threshold (vulgar, hate, sex, self_harm, crime). Empty for allow.';
COMMENT ON COLUMN public.messages.moderation_scanned_at IS
    'When the moderation_scan dispatcher last wrote a verdict. NULL until first scan.';

-- Append-only audit of who tapped to reveal a flagged/blocked message.
-- We need this for misuse detection: a viewer who repeatedly unhides
-- vulgar/hate content is the same signal as a sender who repeatedly
-- posts it. Admin can query this table directly until a UI exists.
--
-- (message_id, viewer_user_id) is the natural PK — revealing twice
-- is idempotent. `revealed_at` reflects the first reveal.
CREATE TABLE IF NOT EXISTS public.chat_message_reveals (
    message_id UUID NOT NULL REFERENCES public.messages(id) ON DELETE CASCADE,
    viewer_user_id INT4 NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    revealed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, viewer_user_id)
);

CREATE INDEX IF NOT EXISTS idx_chat_message_reveals_viewer
    ON public.chat_message_reveals (viewer_user_id, revealed_at DESC);

COMMENT ON TABLE public.chat_message_reveals IS
    'Audit of tap-to-reveal actions on flagged chat messages. Append-only; misuse detection.';

-- RLS: a viewer may insert a reveal only for a message they can see
-- (they're a member of the conversation). They may read their own
-- reveals (UI cache). Admin role bypasses via BYPASSRLS.
ALTER TABLE public.chat_message_reveals ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.chat_message_reveals FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS chat_message_reveals_insert ON public.chat_message_reveals;
CREATE POLICY chat_message_reveals_insert ON public.chat_message_reveals
    FOR INSERT
    WITH CHECK (
        viewer_user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    );

DROP POLICY IF EXISTS chat_message_reveals_select_self ON public.chat_message_reveals;
CREATE POLICY chat_message_reveals_select_self ON public.chat_message_reveals
    FOR SELECT
    USING (viewer_user_id = app.current_user_id());

-- Grants. The api role inserts/selects for the user; worker doesn't
-- touch this table. Wrapped in a role-existence guard so dev DBs
-- (which may not have the least-privilege role created yet) still
-- migrate cleanly.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT INSERT, SELECT ON public.chat_message_reveals TO poziomki_api';
    END IF;
END
$$;
