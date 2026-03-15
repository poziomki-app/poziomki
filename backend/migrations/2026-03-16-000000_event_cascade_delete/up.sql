ALTER TABLE conversations
    DROP CONSTRAINT IF EXISTS conversations_event_id_fkey;

ALTER TABLE conversations
    ADD CONSTRAINT conversations_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;
