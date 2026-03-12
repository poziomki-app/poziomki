ALTER TABLE events
ADD COLUMN max_attendees INTEGER CHECK (max_attendees IS NULL OR max_attendees > 0);
