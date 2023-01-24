-- Add migration script here

DROP INDEX listing_price_per_unit;
DROP INDEX item_info_name;

CREATE INDEX item_info_name on item_info (LOWER(name) text_pattern_ops);
