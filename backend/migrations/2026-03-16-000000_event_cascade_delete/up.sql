ALTER TABLE conversations
    DROP CONSTRAINT IF EXISTS conversations_event_id_fkey;

ALTER TABLE conversations
    ADD CONSTRAINT conversations_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE event_attendees
    DROP CONSTRAINT IF EXISTS event_attendees_event_id_fkey;

ALTER TABLE event_attendees
    ADD CONSTRAINT event_attendees_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;

ALTER TABLE event_tags
    DROP CONSTRAINT IF EXISTS event_tags_event_id_fkey;

ALTER TABLE event_tags
    ADD CONSTRAINT event_tags_event_id_fkey
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE CASCADE;
