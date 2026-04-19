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

-- The admin ban endpoint (backend/src/api/admin.rs) has no viewer
-- context — operators act without a user session — so it can't run
-- the UPDATE + DELETE through with_viewer_tx. Route through a
-- SECURITY DEFINER helper instead: it resolves the target pid,
-- flips the ban columns, and purges sessions in one owner-privileged
-- transaction that bypasses RLS for these narrow writes.
CREATE OR REPLACE FUNCTION app.admin_ban_user(p_user_pid uuid, p_reason text)
RETURNS int
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    v_user_id int;
BEGIN
    SELECT id INTO v_user_id FROM public.users WHERE pid = p_user_pid;
    IF v_user_id IS NULL THEN
        RETURN NULL;
    END IF;

    UPDATE public.users
    SET banned_at = now(),
        banned_reason = p_reason,
        updated_at = now()
    WHERE id = v_user_id;

    DELETE FROM public.sessions WHERE user_id = v_user_id;

    RETURN v_user_id;
END
$$;

COMMENT ON FUNCTION app.admin_ban_user(uuid, text) IS
    'Apply a ban + purge every active session for a user in one owner-privileged transaction. Called by the /api/v1/admin ban endpoint which has no viewer context.';

REVOKE EXECUTE ON FUNCTION app.admin_ban_user(uuid, text) FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT EXECUTE ON FUNCTION app.admin_ban_user(uuid, text) TO poziomki_api';
    END IF;
END
$$;
