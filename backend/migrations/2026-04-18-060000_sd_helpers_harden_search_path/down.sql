-- No-op rollback: re-running the prior migrations' CREATE OR REPLACE
-- statements would restore the pre-hardening search_path. That is a
-- regression we would never want — leave the functions hardened.
SELECT 1;
