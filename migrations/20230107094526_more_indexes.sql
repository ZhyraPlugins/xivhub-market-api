-- Add migration script here

CREATE INDEX upload_item_id ON upload(item_id);
CREATE INDEX upload_world_id ON upload(world_id);

CREATE INDEX listing_world_id ON listing(world_id);
CREATE INDEX listing_price_per_unit ON listing(price_per_unit);

CREATE INDEX purchase_purchase_time ON purchase(purchase_time);
