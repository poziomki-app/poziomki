-- Add ban state to users. Used by the auth middleware + the admin
-- ban endpoint (see backend/src/api/admin/). Columns are NULLable so
-- existing rows don't need a backfill — absence of `banned_at`
-- means the account is active.
ALTER TABLE public.users
    ADD COLUMN IF NOT EXISTS banned_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS banned_reason TEXT;

-- Index to keep the auth-path "is this account active?" check cheap.
-- Partial so it only covers the small banned set, not every row.
CREATE INDEX IF NOT EXISTS idx_users_banned_at
    ON public.users (banned_at)
    WHERE banned_at IS NOT NULL;
