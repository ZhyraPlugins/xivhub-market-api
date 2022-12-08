-- Add migration script here

CREATE INDEX listing_item_id ON listing(item_id);
CREATE INDEX purchase_item_id ON purchase(item_id);
CREATE INDEX purchase_world_id ON purchase(world_id);
CREATE INDEX upload_upload_time ON upload(upload_time);
