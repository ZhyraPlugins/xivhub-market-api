-- Add migration script here

CREATE INDEX IF NOT EXISTS upload_uploader_id ON upload(uploader_id);


-- this is a way faster SELECT COUNT(DISTINCT uploader_id) from upload using a emulated"loose indexscan"
-- https://wiki.postgresql.org/wiki/Loose_indexscan

-- returns the unique count of uploaders
CREATE OR REPLACE VIEW uploader_count AS
    WITH RECURSIVE t AS (
    (SELECT uploader_id FROM upload ORDER BY uploader_id LIMIT 1)  -- parentheses required
    UNION ALL
    SELECT (SELECT uploader_id FROM upload WHERE uploader_id > t.uploader_id ORDER BY uploader_id LIMIT 1)
    FROM t
    WHERE t.uploader_id IS NOT NULL
    )
    SELECT count(uploader_id) FROM t WHERE uploader_id IS NOT NULL;

-- returns the count of different items
CREATE OR REPLACE VIEW unique_items_count AS
    WITH RECURSIVE t AS (
    (SELECT item_id FROM listing ORDER BY item_id LIMIT 1)  -- parentheses required
    UNION ALL
    SELECT (SELECT item_id FROM listing WHERE item_id > t.item_id ORDER BY item_id LIMIT 1)
    FROM t
    WHERE t.item_id IS NOT NULL
    )
    SELECT count(item_id) FROM t WHERE item_id IS NOT NULL;
