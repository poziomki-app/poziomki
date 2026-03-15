ALTER TABLE conversation_members DROP CONSTRAINT IF EXISTS fk_last_read_message;
DROP TABLE IF EXISTS push_subscriptions;
DROP TABLE IF EXISTS message_reactions;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS conversation_members;
DROP TABLE IF EXISTS conversations;
