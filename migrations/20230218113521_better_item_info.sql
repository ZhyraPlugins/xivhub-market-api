-- Add migration script here

ALTER TABLE item_info ADD COLUMN stack_size INT NOT NULL DEFAULT 999;
ALTER TABLE item_info ADD COLUMN level_item INT NOT NULL DEFAULT 1;
ALTER TABLE item_info ADD COLUMN level_equip INT NOT NULL DEFAULT 1;
ALTER TABLE item_info ADD COLUMN materia_slot_count INT NOT NULL DEFAULT 0;
ALTER TABLE item_info ADD COLUMN can_be_hq BOOLEAN NOT NULL DEFAULT FALSE;
