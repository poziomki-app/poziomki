-- Collects free-form test-phase feedback from real users: 1–5 star
-- rating plus optional text. Indexed by created_at so we can scan
-- the latest entries while triaging.

CREATE TABLE public.user_feedback (
    id UUID PRIMARY KEY,
    user_id INT4 NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    rating SMALLINT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    message TEXT,
    app_version TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_user_feedback_created_at ON public.user_feedback (created_at DESC);
CREATE INDEX idx_user_feedback_user_id ON public.user_feedback (user_id);

-- Row-level security: a user can only insert/see their own feedback.
ALTER TABLE public.user_feedback ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_feedback FORCE ROW LEVEL SECURITY;
CREATE POLICY user_feedback_owner ON public.user_feedback
    FOR ALL TO poziomki_api
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());
