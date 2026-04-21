DO $$
DECLARE
    api_role text := COALESCE(current_setting('app.api_role', true), 'poziomki_api');
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = api_role) THEN
        BEGIN
            EXECUTE format('ALTER ROLE %I RESET statement_timeout', api_role);
        EXCEPTION WHEN insufficient_privilege THEN
            RAISE NOTICE 'skipped ALTER ROLE % RESET statement_timeout: insufficient privilege', api_role;
        END;
    END IF;
END
$$;

-- Re-grant the PG default so rollback lands the cluster back where it
-- was regardless of which PG version installed the original default.
GRANT USAGE ON SCHEMA public TO PUBLIC;

DROP TRIGGER IF EXISTS user_settings_audit ON public.user_settings;
DROP TRIGGER IF EXISTS users_audit ON public.users;
DROP FUNCTION IF EXISTS audit.log_change();
DROP TABLE IF EXISTS audit.events;
DROP SCHEMA IF EXISTS audit CASCADE;

ALTER TABLE public.conversations
    DROP CONSTRAINT IF EXISTS event_chat_pair_null;
ALTER TABLE public.conversations
    DROP CONSTRAINT IF EXISTS dm_canonical_pair;
