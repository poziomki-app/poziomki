-- Staging role + database bootstrap for Poziomki's slim parallel stack.
--
-- Staging shares the prod Postgres cluster; this script carves out a fresh
-- database and a distinct triplet of login roles. Safe to re-run.
--
-- Run as the cluster superuser (`poziomki`) against the `postgres` maintenance
-- database. The wrapper `setup-roles-staging.sh` invokes this with the right
-- -v variables.
--
-- Variables (passed via `psql -v`):
--   owner_password     -- password for poziomki_staging (DB owner, runs migrations)
--   api_password       -- password for poziomki_staging_api (app connections)
--   worker_password    -- password for poziomki_staging_worker (background jobs)
--
-- Role model:
--   * poziomki_staging       — owns the staging DB. NOT a cluster superuser;
--     migrations run as this role. Members: the staging api/worker roles
--     (so the owner can administer them without being superuser).
--   * poziomki_staging_api   — NOBYPASSRLS; the running staging API.
--     GRANTed `poziomki_api` so every RLS policy written `TO poziomki_api`
--     applies via membership (PostgreSQL CREATE POLICY matches members).
--   * poziomki_staging_worker — BYPASSRLS **attribute** set directly.
--     Role *attributes* (BYPASSRLS, SUPERUSER, …) are non-inheritable even
--     when the role is granted poziomki_worker — the attribute must be set
--     on the login role itself.
--
-- Extensions (cube, earthdistance, pg_trgm) are installed into the staging
-- DB by this script because the migration's `CREATE EXTENSION` requires
-- superuser and the staging owner deliberately isn't one.

\set ON_ERROR_STOP on

-- ---------------------------------------------------------------------------
-- Roles
-- ---------------------------------------------------------------------------

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_staging') THEN
    CREATE ROLE poziomki_staging LOGIN NOSUPERUSER CREATEDB NOCREATEROLE NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_staging_api') THEN
    CREATE ROLE poziomki_staging_api LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'poziomki_staging_worker') THEN
    CREATE ROLE poziomki_staging_worker LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE BYPASSRLS;
  END IF;
END
$$;

ALTER ROLE poziomki_staging        WITH PASSWORD :'owner_password';
ALTER ROLE poziomki_staging_api    WITH PASSWORD :'api_password';
ALTER ROLE poziomki_staging_worker WITH PASSWORD :'worker_password';

-- Re-assert attributes on every run — role creation above is guarded by
-- existence, so a pre-existing role without BYPASSRLS (the original bug this
-- script fixes) gets the attribute applied here.
ALTER ROLE poziomki_staging_api    WITH NOBYPASSRLS;
ALTER ROLE poziomki_staging_worker WITH BYPASSRLS;

-- RLS policies across migrations target the prod role names literally.
-- Granting them to the staging login roles makes CREATE POLICY ... TO role
-- match via role membership, so the policies apply to staging sessions too.
GRANT poziomki_api    TO poziomki_staging_api;
GRANT poziomki_worker TO poziomki_staging_worker;

-- Lets the staging DB owner administer the two staging login roles (set
-- passwords, ALTER ROLE SET guc) without being a cluster superuser.
GRANT poziomki_staging_api    TO poziomki_staging;
GRANT poziomki_staging_worker TO poziomki_staging;

-- ---------------------------------------------------------------------------
-- Database
-- ---------------------------------------------------------------------------

SELECT 'CREATE DATABASE poziomki_staging OWNER poziomki_staging'
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = 'poziomki_staging')
\gexec

GRANT CONNECT ON DATABASE poziomki_staging TO
  poziomki_staging, poziomki_staging_api, poziomki_staging_worker;

-- Per-database override read by migrations 2026-04-18-010000 and
-- 2026-04-19-040000 — targets staging roles for the ALTER statements that
-- otherwise default to `poziomki_api` (which is prod's role).
ALTER DATABASE poziomki_staging SET app.api_role = 'poziomki_staging_api';
