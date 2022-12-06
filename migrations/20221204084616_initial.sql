-- Add migration script here

CREATE TABLE upload (
    id UUID NOT NULL PRIMARY KEY,
    uploader_id TEXT NOT NULL,
    upload_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    world_id INT NOT NULL,
    item_id INT NOT NULL,
    -- 1 = listing, 2 = history
    upload_type INT NOT NULL
);

CREATE TABLE listing (
    listing_id BIGINT NOT NULL,
    upload_id UUID NOT NULL,
    world_id INT NOT NULL,
    item_id INT NOT NULL,

    hq BOOLEAN NOT NULL,
    seller_id TEXT NOT NULL,
    retainer_id TEXT NOT NULL,
    retainer_name TEXT NULL,
    creator_id TEXT NOT NULL,
    creator_name TEXT NULL,

    last_review_time TIMESTAMP WITH TIME ZONE NOT NULL,
    price_per_unit INT NOT NULL,
    quantity INT NOT NULL,
    
    retainer_city_id INT NOT NULL,
    materia_count INT NOT NULL
);

-- https://github.com/goatcorp/Dalamud/blob/b82c2d8766b371e3539968d5d5e833b11c1fcf3d/Dalamud/Game/Network/Structures/MarketBoardHistory.cs#L78
CREATE TABLE purchase (
    item_id INT NOT NULL,
    world_id INT NOT NULL,
    upload_id UUID NOT NULL,
    buyer_name TEXT NOT NULL,
    hq BOOLEAN NOT NULL,
    on_mannequin BOOLEAN NOT NULL,
    purchase_time TIMESTAMP WITH TIME ZONE NOT NULL,
    quantity INT NOT NULL,
    price_per_unit INT NOT NULL
);

CREATE INDEX purchase_idx_purchase_time ON purchase(item_id, purchase_time);
