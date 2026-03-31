ALTER TABLE conversation_members DROP COLUMN IF EXISTS archived_at;
DROP TABLE IF EXISTS profile_blocks CASCADE;
