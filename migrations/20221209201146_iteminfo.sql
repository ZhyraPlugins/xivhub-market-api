-- Add migration script here

CREATE TABLE item_info (
    item_id INT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    icon TEXT NOT NULL,
    icon_hd TEXT NOT NULL,
    description TEXT NOT NULL,
    item_kind_name TEXT NOT NULL,
    item_kind_id INT NOT NULL,
    item_search_category INT NOT NULL,
    item_search_category_iconhd TEXT NOT NULL,
    item_search_category_name TEXT NOT NULL
);

CREATE INDEX item_info_name ON item_info(name);
CREATE INDEX item_info_category ON item_info(item_search_category);
