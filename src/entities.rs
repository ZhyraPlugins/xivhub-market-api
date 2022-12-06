use chrono::{DateTime, Utc};
use serde::{Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Upload {
    pub id: Uuid,
    pub uploader_id: String,
    pub upload_time: DateTime<Utc>,
    pub world_id: i32,
    pub item_id: i32,
    pub upload_type: i32,
}

#[derive(Debug, Serialize)]
pub struct Listing {
    pub listing_id: i64,
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