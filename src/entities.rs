use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Upload {
    pub id: Uuid,
    pub uploader_id: String,
    pub upload_time: DateTime<Utc>,
    pub world_id: i32,
    pub item_id: i32,
    pub upload_type: i32,
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Serialize)]
pub struct Listing {
    pub upload_id: Uuid,
    pub world_id: i32,
    pub item_id: i32,
    pub hq: bool,
    pub seller_id: String,
    pub retainer_id: String,
    pub retainer_name: Option<String>,
    pub creator_id: String,
    pub creator_name: Option<String>,
    pub last_review_time: DateTime<Utc>,
    pub price_per_unit: i32,
    pub quantity: i32,
    pub retainer_city_id: i32,
    pub materia_count: i32,
}

#[derive(Debug, Serialize)]
pub struct Purchase {
    pub item_id: i32,
    pub world_id: i32,
    pub upload_id: Uuid,
    pub buyer_name: String,
    pub hq: bool,
    pub on_mannequin: bool,
    pub purchase_time: DateTime<Utc>,
    pub quantity: i32,
    pub price_per_unit: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemInfo {
    pub item_id: i32,
    pub name: String,
    pub icon: String,
    pub icon_hd: String,
    pub description: String,
    pub item_kind_name: String,
    pub item_kind_id: i32,
    pub item_search_category: i32,
    pub item_search_category_iconhd: String,
    pub item_search_category_name: String,
    pub stack_size: i32,
    pub level_item: i32,
    pub level_equip: i32,
    pub materia_slot_count: i32,
    pub rarity: i32,
    pub can_be_hq: bool,
}
