use crate::entities::Upload;
use axum::{extract::State, Json};
use axum_prometheus::metrics::{histogram, increment_counter};
use chrono::TimeZone;
use color_eyre::eyre::eyre;
use serde::Deserialize;
use std::time::Instant;
use tracing::info;
use uuid::Uuid;

use crate::{error::AppError, AppState};

#[derive(Debug, Deserialize)]
pub struct Request<T> {
    pub world_id: i32,
    pub item_id: i32,
    pub uploader_id: String,
    pub listings: Vec<T>,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct RequestListing {
    pub hq: bool,
    pub seller_id: String,
    pub retainer_id: String,
    pub retainer_name: String,
    pub creator_id: String, // artisan
    pub creator_name: String,
    pub on_mannequin: bool,
    pub last_review_time: i64,
    pub price_per_unit: i32,
    pub quantity: i32,
    pub retainer_city: i32,
    pub materia: Vec<ItemMateria>,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct ItemMateria {
    pub slot_id: i32,
    pub materia_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct HistoryRequestListing {
    pub hq: bool,
    pub buyer_name: String,
    pub on_mannequin: bool,
    pub purchase_time: i64,
    pub price_per_unit: i32,
    pub quantity: i32,
}

pub async fn listings(
    State(state): State<AppState>,
    Json(payload): Json<Request<RequestListing>>,
) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    let date = chrono::Utc::now();
    info!("Received upload for item {}", payload.item_id);

    if payload.world_id > 1000 || payload.world_id == 0 {
        // mostly chinese servers
        // todo: handle better
        return Ok(());
    }

    let upload_time = Instant::now();

    sqlx::query!(
        "INSERT INTO upload (id, uploader_id, upload_time, world_id, item_id, upload_type)
        VALUES ($1,$2,$3,$4,$5,$6)",
        id,
        payload.uploader_id,
        date,
        payload.world_id,
        payload.item_id,
        0
    )
    .execute(&state.pool)
    .await?;

    // for now, dont keep a history of previous listings.
    let mut rows_affected = sqlx::query!(
        "DELETE FROM listing WHERE item_id = $1 AND world_id =$2",
        payload.item_id,
        payload.world_id
    )
    .execute(&state.pool)
    .await?
    .rows_affected();

    for listing in payload.listings {
        let date = chrono::Utc
            .timestamp_opt(listing.last_review_time, 0)
            .single()
            .ok_or_else(|| AppError(eyre!("invalid last_review_time date")))?;

        let materia_count: i32 = listing.materia.len().try_into()?;
        rows_affected += sqlx::query!(
            "INSERT INTO listing (
                upload_id, world_id, item_id, seller_id,
                retainer_id, retainer_name, creator_id, creator_name,
                last_review_time, price_per_unit, quantity,
                retainer_city_id, materia_count, hq)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            ",
            id,
            payload.world_id,
            payload.item_id,
            listing.seller_id,
            listing.retainer_id,
            listing.retainer_name,
            listing.creator_id,
            listing.creator_name,
            date,
            listing.price_per_unit,
            listing.quantity,
            listing.retainer_city,
            materia_count,
            listing.hq,
        )
        .execute(&state.pool)
        .await?
        .rows_affected();
    }

    let upload_time_elapsed = upload_time.elapsed();

    increment_counter!("xivhub_update", "type" => "listings");
    histogram!("xivhub_update_time", upload_time_elapsed, "type" => "listings");

    if rows_affected > 0 {
        state.item_listings_cache.invalidate(&payload.item_id).await;
    }

    Ok(())
}

pub async fn history(
    State(state): State<AppState>,
    Json(payload): Json<Request<HistoryRequestListing>>,
) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    let date = chrono::Utc::now();
    info!(
        "Received purchase history upload for item {}",
        payload.item_id
    );

    if payload.world_id > 1000 || payload.world_id == 0 {
        // mostly chinese servers
        // todo: handle better
        return Ok(());
    }

    let mut trans = state.pool.begin().await?;

    sqlx::query!(
        "INSERT INTO upload (id, uploader_id, upload_time, world_id, item_id, upload_type)
        VALUES ($1,$2,$3,$4,$5,$6)",
        id,
        payload.uploader_id,
        date,
        payload.world_id,
        payload.item_id,
        1
    )
    .execute(&mut trans)
    .await?;

    let mut rows_affected = 0;

    // If iter is not emtpy returns some.
    if let Some(oldest_purchase) = payload.listings.iter().map(|x| x.purchase_time).min() {
        let oldest_date = chrono::Utc
            .timestamp_opt(oldest_purchase, 0)
            .single()
            .ok_or_else(|| AppError(eyre!("invalid oldest purchase date")))?;

        // delete records more recent than the last purchase time
        rows_affected = sqlx::query!(
            "DELETE FROM purchase WHERE item_id = $1 AND world_id = $2 AND purchase_time >= $3",
            payload.item_id,
            payload.world_id,
            oldest_date
        )
        .execute(&mut trans)
        .await?
        .rows_affected();

        for listing in payload.listings {
            let date = chrono::Utc
                .timestamp_opt(listing.purchase_time, 0)
                .single()
                .ok_or_else(|| AppError(eyre!("invalid purchase_time")))?;

            rows_affected = sqlx::query!(
                "INSERT INTO purchase (
                    upload_id, item_id, world_id, buyer_name, hq, on_mannequin, purchase_time, quantity, price_per_unit)
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                ",
                id,
                payload.item_id,
                payload.world_id,
                listing.buyer_name,
                listing.hq,
                listing.on_mannequin,
                date,
                listing.quantity,
                listing.price_per_unit
            )
            .execute(&mut trans)
            .await?.rows_affected();
        }
    }

    let upload_time = Instant::now();

    trans.commit().await?;

    let upload_time_elapsed = upload_time.elapsed();

    increment_counter!("xivhub_update", "type" => "history");
    histogram!("xivhub_query", upload_time_elapsed, "type" => "history");

    if rows_affected > 0 {
        state.item_purchase_cache.invalidate(&payload.item_id).await;
    }
    Ok(())
}

pub async fn last_uploads(State(state): State<AppState>) -> Result<Json<Vec<Upload>>, AppError> {
    let start = Instant::now();
    let uploads = sqlx::query_as!(
        Upload,
        "SELECT u.*, f.name, f.icon FROM upload u LEFT JOIN item_info f ON f.item_id = u.item_id WHERE upload_type = 0 ORDER BY upload_time DESC LIMIT 250"
    )
    .fetch_all(&state.pool)
    .await?;
    let elapsed = start.elapsed();
    histogram!("xivhub_query", elapsed, "type" => "last_uploads");

    Ok(Json(uploads))
}
