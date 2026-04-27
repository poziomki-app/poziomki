-- Per-message report queue.
--
-- Conversation-level reports already exist (chat.report_handler →
-- conversations_reports). Message-level reports are different:
-- they're typically driven by a moderation flag (Bielik-Guard hit
-- the message, the user revealed it, then chose to escalate). The
-- row snapshots the moderation context at report time so admin
-- can tell the difference between "model false positive" and
-- "model caught it but it's still harassment".
--
-- (message_id, reporter_user_id) is the natural PK — a viewer
-- reporting twice is idempotent.

CREATE TABLE IF NOT EXISTS public.chat_message_reports (
    message_id UUID NOT NULL REFERENCES public.messages(id) ON DELETE CASCADE,
    reporter_user_id INT4 NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    description TEXT,
    -- Snapshot of the moderation verdict + categories at the moment
    -- the user filed the report, so admin sees what the auto-mod
    -- engine thought regardless of whether the row changes later.
    automoderation_verdict TEXT,
    automoderation_categories TEXT[] NOT NULL DEFAULT '{}'::text[],
    status TEXT NOT NULL DEFAULT 'open',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, reporter_user_id)
);

CREATE INDEX IF NOT EXISTS idx_chat_message_reports_status_created
    ON public.chat_message_reports (status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_chat_message_reports_reporter
    ON public.chat_message_reports (reporter_user_id, created_at DESC);

COMMENT ON TABLE public.chat_message_reports IS
    'Per-message moderation reports. Snapshots the auto-mod verdict at file time so admin can see model-vs-human disagreement.';

-- RLS: a viewer can insert a report only for a message they can
-- actually see (same gate as chat_message_reveals); they can read
-- their own reports for UI cache. Admin role bypasses via BYPASSRLS.
ALTER TABLE public.chat_message_reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.chat_message_reports FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS chat_message_reports_insert ON public.chat_message_reports;
CREATE POLICY chat_message_reports_insert ON public.chat_message_reports
    FOR INSERT
    WITH CHECK (
        reporter_user_id = app.current_user_id()
        AND app.viewer_can_see_message(message_id)
    );

DROP POLICY IF EXISTS chat_message_reports_select_self ON public.chat_message_reports;
CREATE POLICY chat_message_reports_select_self ON public.chat_message_reports
    FOR SELECT
    USING (reporter_user_id = app.current_user_id());

-- Grants. The api role inserts/selects on the user's behalf;
-- worker doesn't write here. Admin role is BYPASSRLS so doesn't
-- need an explicit grant.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT INSERT, SELECT ON public.chat_message_reports TO poziomki_api';
    END IF;
END
$$;
