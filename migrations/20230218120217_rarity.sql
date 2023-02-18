-- Add migration script here

ALTER TABLE item_info ADD COLUMN rarity INT NOT NULL DEFAULT 1;
