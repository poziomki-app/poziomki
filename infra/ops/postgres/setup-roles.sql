-- One-time role + privilege bootstrap for Poziomki.
--
-- Creates the least-privilege roles the API and worker will connect as, so the
-- application is no longer the cluster superuser. This is the Phase 1 baseline
-- for Row-Level Security: no policies yet, just role separation.
--
-- Run as the database owner (the existing `poziomki` superuser) against the
-- application database (`poziomki-rs` in production). The wrapper script
-- `setup-roles.sh` invokes this with the right -v variables.
--
-- Safe to re-run: all statements are idempotent. Re-running rotates passwords
-- to whatever you pass in.
--
-- Variables (passed via `psql -v`):
--   api_password      -- password for poziomki_api
--   worker_password   -- password for poziomki_worker

\set ON_ERROR_STOP on

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_api') THEN
    CREATE ROLE poziomki_api LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_worker') THEN
    -- Worker legitimately crosses user boundaries (outbox dispatch, cleanup).
    -- Give it BYPASSRLS so it isn't tripped by the policies we add in Phase 3.
    CREATE ROLE poziomki_worker LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE BYPASSRLS;
  END IF;
END
$$;

-- Passwords come from psql variables; quoted via :'...' to escape safely.
ALTER ROLE poziomki_api     WITH PASSWORD :'api_password';
ALTER ROLE poziomki_worker  WITH PASSWORD :'worker_password';

-- Bound the worst-case query cost for an exploited or buggy API endpoint.
ALTER ROLE poziomki_api SET statement_timeout = '5s';

-- ---------------------------------------------------------------------------
-- Privileges on the current schema
-- ---------------------------------------------------------------------------

GRANT USAGE ON SCHEMA public TO poziomki_api, poziomki_worker;

GRANT SELECT, INSERT, UPDATE, DELETE
  ON ALL TABLES IN SCHEMA public
  TO poziomki_api, poziomki_worker;

GRANT USAGE, SELECT
  ON ALL SEQUENCES IN SCHEMA public
  TO poziomki_api, poziomki_worker;

-- ---------------------------------------------------------------------------
-- Default privileges for future migrations
--
-- Applies whenever the owner (`poziomki`) creates a new table or sequence in
-- `public`, so we don't have to re-run grants after every migration.
-- ---------------------------------------------------------------------------

ALTER DEFAULT PRIVILEGES FOR ROLE poziomki IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES
  TO poziomki_api, poziomki_worker;

ALTER DEFAULT PRIVILEGES FOR ROLE poziomki IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES
  TO poziomki_api, poziomki_worker;
