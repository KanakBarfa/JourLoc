CREATE TABLE IF NOT EXISTS pages (
  id SERIAL PRIMARY KEY,
  title TEXT NOT NULL,
  content TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS tags (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS page_tags (
  page_id INTEGER NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
  tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (page_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_pages_title ON pages (LOWER(title));
CREATE INDEX IF NOT EXISTS idx_tags_name ON tags (LOWER(name));

CREATE OR REPLACE FUNCTION cleanup_orphan_tags() RETURNS trigger AS $$
BEGIN
  DELETE FROM tags t
  WHERE NOT EXISTS (
    SELECT 1 FROM page_tags pt WHERE pt.tag_id = t.id
  );
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_cleanup_orphan_tags ON page_tags;
CREATE TRIGGER trg_cleanup_orphan_tags
AFTER DELETE OR UPDATE OF tag_id ON page_tags
FOR EACH STATEMENT
EXECUTE FUNCTION cleanup_orphan_tags();
