use crate::{
    entities::{ItemInfo, Listing, Purchase},
    error::AppError,
    util::fetch_item_info,
    AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use metrics::{histogram, increment_counter};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Clone)]
pub struct ListingsResponse {
    pub item: ItemInfo,
    pub listings: Vec<Listing>,
}

#[derive(Debug, Deserialize)]
pub struct ListingsQuery {}

pub async fn listings(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
) -> Result<Json<ListingsResponse>, AppError> {
    increment_counter!("xivhub_listings_request");

    let listings_time = Instant::now();

    let listings = state
        .item_listings_cache
        .try_get_with(item_id, async {
            increment_counter!("xivhub_listings_request_cache_miss");
            let listings = sqlx::query_as!(
            Listing,
            "SELECT * FROM listing WHERE item_id = $1 ORDER BY world_id ASC, price_per_unit ASC",
            item_id
        )
            .fetch_all(&state.pool)
            .await?;

            let item = fetch_item_info(item_id, &state.pool).await?;

            Ok::<_, sqlx::Error>(ListingsResponse { item, listings })
        })
        .await?;

    let listings_time = listings_time.elapsed();
    histogram!("xivhub_get_item_listings_time", listings_time);

    Ok(Json(listings))
}

#[derive(Debug, Serialize, Clone)]
pub struct PurchasesResponse {
    pub item: ItemInfo,
    pub page: i64,
    pub purchases: Vec<Purchase>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct PurchasesQuery {
    pub page: Option<i64>,
}

pub async fn purchases(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Query(query): Query<PurchasesQuery>,
) -> Result<Json<PurchasesResponse>, AppError> {
    let page = query.page.unwrap_or(0);
    increment_counter!("xivhub_purchases_request", "page" => page.to_string());

    // only cache page 0.
    let purchases = if page == 0 {
        state.item_purchase_cache.try_get_with(item_id, async {
            increment_counter!("xivhub_purchases_request_cache_miss", "page" => page.to_string());

            let start = Instant::now();
            let purchases = sqlx::query_as!(
                    Purchase,
                    "SELECT * FROM purchase WHERE item_id = $1 ORDER BY purchase_time DESC OFFSET $2 LIMIT $3",
                    item_id,
                    page * 250,
                    250
            )
            .fetch_all(&state.pool)
            .await?;
            let elapsed = start.elapsed();
            histogram!("xivhub_query", elapsed, "type" => "purchases");

            let item = fetch_item_info(item_id, &state.pool).await?;
            Ok::<_, sqlx::Error>(PurchasesResponse {
                item,
                page,
                purchases,
            })
        }).await?
    } else {
        increment_counter!("xivhub_purchases_request_cache_miss", "page" => page.to_string());

        let start = Instant::now();
        let purchases = sqlx::query_as!(
            Purchase,
            "SELECT * FROM purchase WHERE item_id = $1 ORDER BY purchase_time DESC OFFSET $2 LIMIT $3",
            item_id,
            page * 250,
            250
        )
        .fetch_all(&state.pool)
        .await?;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "purchases");

        let item = fetch_item_info(item_id, &state.pool).await?;
        PurchasesResponse {
            item,
            page,
            purchases,
        }
    };

    Ok(Json(purchases))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ItemUploadDates {
    pub world_id: i32,
    pub upload_time: Option<DateTime<Utc>>,
}

/// returns the last upload dates per world for an item
pub async fn get_item_upload_dates(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
) -> Result<Json<Vec<ItemUploadDates>>, AppError> {
    let start = Instant::now();
    let uploads = sqlx::query_as!(
        ItemUploadDates,
        "SELECT world_id, MAX(upload_time) as upload_time FROM upload WHERE item_id = $1 GROUP BY world_id, item_id",
        item_id
    )
    .fetch_all(&state.pool)
    .await?;
    let elapsed = start.elapsed();
    histogram!("xivhub_query", elapsed, "type" => "item_upload_dates");

    Ok(Json(uploads))
}

#[derive(Debug, Deserialize)]
pub struct ItemListQuery {
    pub page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ItemList {
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
    pub listings: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListItemsResponse {
    pub items: Vec<ItemList>,
    pub page: i64,
    pub total_pages: i64,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ItemListQuery>,
) -> Result<Json<ListItemsResponse>, AppError> {
    let page = query.page.unwrap_or(0);

    let start = Instant::now();
    let total_items = {
        if let Some(search) = &query.search {
            sqlx::query!(
                "SELECT COUNT(*) from item_info WHERE LOWER(name) LIKE LOWER($1)",
                format!("%{search}%")
            )
            .fetch_one(&state.pool)
            .await?
            .count
        } else {
            sqlx::query!("SELECT COUNT(*) from item_info")
                .fetch_one(&state.pool)
                .await?
                .count
        }
    }
    .unwrap_or(0);
    let elapsed = start.elapsed();
    histogram!("xivhub_query", elapsed, "type" => "list_items_total", "search" => query.search.is_some().to_string());

    let start = Instant::now();
    let items = {
        if let Some(search) = &query.search {
            sqlx::query_as!(
                ItemList,
                "SELECT i.*, (SELECT COUNT(*) FROM listing l WHERE l.item_id = i.item_id) as listings
                FROM item_info i
                WHERE LOWER(name) LIKE LOWER($1)
                ORDER BY item_id ASC
                OFFSET $2
                LIMIT 100
                ",
                format!("%{search}%"),
                page * 100
            )
            .fetch_all(&state.pool)
            .await?
        } else {
            sqlx::query_as!(
                ItemList,
                "SELECT i.*, (SELECT COUNT(*) FROM listing l WHERE l.item_id = i.item_id) as listings
                FROM item_info i
                ORDER BY item_id ASC
                OFFSET $1
                LIMIT 100
                ",
                page * 50
            )
            .fetch_all(&state.pool)
            .await?
        }
    };
    let elapsed = start.elapsed();
    histogram!("xivhub_query", elapsed, "type" => "list_item", "search" => query.search.is_some().to_string());

    Ok(Json(ListItemsResponse {
        items,
        page,
        total_pages: total_items / 50,
    }))
}
