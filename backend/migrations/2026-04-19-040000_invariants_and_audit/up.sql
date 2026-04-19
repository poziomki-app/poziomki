-- Final phase: DB-level invariants + generic audit trigger + schema
-- hardening. Complements the Tier A/B/C policies by locking
-- assumptions the RLS policies already rely on (e.g. DM pair is
-- canonical + distinct) and gives forensic coverage over the two
-- highest-sensitivity mutation surfaces: users.email /
-- users.password and user_settings. Finally tightens the schema's
-- default privileges.

-- ---------------------------------------------------------------------------
-- DM shape invariants on conversations. Tier-B's conversations_insert
-- policy already enforces the same predicate for poziomki_api, but the
-- owner role + migrations + future BYPASSRLS callers need the same
-- invariant too. Declare it as a CHECK so it can never be bypassed.
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'dm_canonical_pair'
          AND conrelid = 'public.conversations'::regclass
    ) THEN
        ALTER TABLE public.conversations
            ADD CONSTRAINT dm_canonical_pair
            CHECK (
                kind <> 'dm'
                OR (user_low_id IS NOT NULL
                    AND user_high_id IS NOT NULL
                    AND user_low_id < user_high_id)
            );
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'event_chat_pair_null'
          AND conrelid = 'public.conversations'::regclass
    ) THEN
        ALTER TABLE public.conversations
            ADD CONSTRAINT event_chat_pair_null
            CHECK (
                kind <> 'event'
                OR (user_low_id IS NULL AND user_high_id IS NULL)
            );
    END IF;
END
$$;

-- ---------------------------------------------------------------------------
-- audit.events — generic forensic log. Captures the columns that
-- changed, old + new JSONB snapshots, the viewer id from the GUC,
-- and the SQL operation. Only attached to the handful of tables
-- where after-the-fact review matters (users, user_settings).
-- ---------------------------------------------------------------------------
CREATE SCHEMA IF NOT EXISTS audit;

CREATE TABLE IF NOT EXISTS audit.events (
    id BIGSERIAL PRIMARY KEY,
    table_name TEXT NOT NULL,
    row_pk TEXT NOT NULL,
    op CHAR(1) NOT NULL CHECK (op IN ('I', 'U', 'D')),
    changed_columns TEXT[],
    old_data JSONB,
    new_data JSONB,
    actor_user_id INT4,
    session_user_name TEXT NOT NULL DEFAULT session_user,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_events_table_row
    ON audit.events (table_name, row_pk, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_created_at
    ON audit.events (created_at DESC);

-- Revoke public access; the table is INSERT-only for the app roles
-- (the trigger function writes it) and SELECT-only for the owner.
REVOKE ALL ON audit.events FROM PUBLIC;
REVOKE ALL ON SCHEMA audit FROM PUBLIC;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA audit TO poziomki_api';
        EXECUTE 'GRANT INSERT ON audit.events TO poziomki_api';
        EXECUTE 'GRANT USAGE, SELECT ON SEQUENCE audit.events_id_seq TO poziomki_api';
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_worker') THEN
        EXECUTE 'GRANT USAGE ON SCHEMA audit TO poziomki_worker';
        EXECUTE 'GRANT INSERT ON audit.events TO poziomki_worker';
        EXECUTE 'GRANT USAGE, SELECT ON SEQUENCE audit.events_id_seq TO poziomki_worker';
    END IF;
END
$$;

-- Generic audit trigger function. SECURITY DEFINER so it can always
-- write to audit.events even when the caller's role lacks INSERT
-- (future-proofs against new least-privilege roles).
CREATE OR REPLACE FUNCTION audit.log_change()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    v_row_pk          text;
    v_op              char(1);
    v_old_jsonb       jsonb;
    v_new_jsonb       jsonb;
    v_changed         text[];
    v_actor           int;
    v_sensitive_cols  text[] := ARRAY[]::text[];
BEGIN
    -- Never persist credential material in audit.events. We still
    -- surface the column in `changed_columns` (so "password rotated"
    -- is a visible event), but the JSONB snapshots get the sensitive
    -- keys stripped before storage.
    IF TG_TABLE_NAME = 'users' THEN
        v_sensitive_cols := ARRAY[
            'password',
            'api_key',
            'reset_token',
            'email_verification_token',
            'magic_link_token'
        ];
    END IF;

    IF TG_OP = 'INSERT' THEN
        v_op := 'I';
        v_old_jsonb := NULL;
        v_new_jsonb := to_jsonb(NEW);
        v_changed := NULL;
        v_row_pk := (v_new_jsonb ->> 'id');
    ELSIF TG_OP = 'UPDATE' THEN
        v_op := 'U';
        v_old_jsonb := to_jsonb(OLD);
        v_new_jsonb := to_jsonb(NEW);
        SELECT ARRAY(
            SELECT key
            FROM jsonb_each(v_new_jsonb)
            WHERE v_new_jsonb -> key IS DISTINCT FROM v_old_jsonb -> key
        ) INTO v_changed;
        IF array_length(v_changed, 1) IS NULL THEN
            RETURN NEW;
        END IF;
        v_row_pk := (v_new_jsonb ->> 'id');
    ELSIF TG_OP = 'DELETE' THEN
        v_op := 'D';
        v_old_jsonb := to_jsonb(OLD);
        v_new_jsonb := NULL;
        v_changed := NULL;
        v_row_pk := (v_old_jsonb ->> 'id');
    END IF;

    -- Strip sensitive keys from the stored payload (changed_columns
    -- is computed above against the unredacted row so rotation
    -- events still surface).
    IF array_length(v_sensitive_cols, 1) IS NOT NULL THEN
        IF v_old_jsonb IS NOT NULL THEN
            v_old_jsonb := v_old_jsonb - v_sensitive_cols;
        END IF;
        IF v_new_jsonb IS NOT NULL THEN
            v_new_jsonb := v_new_jsonb - v_sensitive_cols;
        END IF;
    END IF;

    BEGIN
        v_actor := NULLIF(current_setting('app.user_id', true), '')::int;
    EXCEPTION WHEN OTHERS THEN
        v_actor := NULL;
    END;

    INSERT INTO audit.events
        (table_name, row_pk, op, changed_columns, old_data, new_data, actor_user_id)
    VALUES
        (TG_TABLE_SCHEMA || '.' || TG_TABLE_NAME,
         COALESCE(v_row_pk, ''),
         v_op,
         v_changed,
         v_old_jsonb,
         v_new_jsonb,
         v_actor);

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    ELSE
        RETURN NEW;
    END IF;
END
$$;

COMMENT ON FUNCTION audit.log_change() IS
    'Generic audit trigger — captures op, changed columns, old/new JSONB snapshots, and the viewer id from the app.user_id GUC. Attach via AFTER INSERT/UPDATE/DELETE FOR EACH ROW.';

REVOKE EXECUTE ON FUNCTION audit.log_change() FROM PUBLIC;

-- users: track INSERT + UPDATE + DELETE. INSERT is rare (signup); the
-- point of interest is UPDATE on email / password_hash plus DELETE
-- (account termination). The trigger captures every column, but the
-- `changed_columns` set lets readers filter for the sensitive ones.
DROP TRIGGER IF EXISTS users_audit ON public.users;
CREATE TRIGGER users_audit
    AFTER INSERT OR UPDATE OR DELETE ON public.users
    FOR EACH ROW
    EXECUTE FUNCTION audit.log_change();

-- user_settings: privacy toggles, notifications flag, etc. All
-- changes worth auditing.
DROP TRIGGER IF EXISTS user_settings_audit ON public.user_settings;
CREATE TRIGGER user_settings_audit
    AFTER INSERT OR UPDATE OR DELETE ON public.user_settings
    FOR EACH ROW
    EXECUTE FUNCTION audit.log_change();

-- ---------------------------------------------------------------------------
-- Schema hardening. `REVOKE ALL ON SCHEMA public FROM PUBLIC` is the
-- PG15 default but not always applied to older clusters; asserting it
-- is cheap and stops new roles from inheriting default creation
-- privileges. Follow with an explicit statement_timeout on the API
-- role so a single runaway query can't tie up the pool.
-- ---------------------------------------------------------------------------
REVOKE ALL ON SCHEMA public FROM PUBLIC;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
        EXECUTE 'ALTER ROLE poziomki_api SET statement_timeout = ''5s''';
    END IF;
END
$$;
