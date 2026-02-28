ALTER TABLE tags
    ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('simple', COALESCE(name, ''))) STORED;

CREATE INDEX idx_tags_fts ON tags USING GIN (search_vector);
