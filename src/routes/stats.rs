use crate::error::AppError;
use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::AppState;

#[derive(Debug, Serialize, Clone)]
pub struct Stats {
    pub total_uploads: i64,
    pub active_listings: i64,
    pub total_purchases: i64,
    pub unique_uploaders: i64,
    pub unique_items: i64,
    pub uploads_per_day: Vec<DayCount>,
    pub purchase_by_day: Vec<DayCount>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DayCount {
    pub count: Option<i64>,
    pub day: Option<DateTime<Utc>>,
}

pub async fn stats(State(state): State<AppState>) -> Result<Json<Stats>, AppError> {
    let stats_value = state.stats_cache.try_get_with((), async {
        let uploads = sqlx::query!("SELECT COUNT(*) from upload")
            .fetch_one(&state.pool)
            .await?;

        let active_listings = sqlx::query!("SELECT COUNT(*) from listing")
            .fetch_one(&state.pool)
            .await?;

        let purchases = sqlx::query!("SELECT COUNT(*) from purchase")
            .fetch_one(&state.pool)
            .await?;

        let unique_uploaders = sqlx::query!("SELECT count FROM uploader_count")
            .fetch_one(&state.pool)
            .await?;

        let unique_items = sqlx::query!("SELECT count from unique_items_count")
            .fetch_one(&state.pool)
            .await?;

        let mut uploads_per_day = sqlx::query_as!(DayCount,
            "SELECT COUNT(*) as count, DATE_TRUNC('day', upload_time) as day from upload GROUP BY DATE_TRUNC('day', upload_time) ORDER BY day DESC LIMIT 15")
            .fetch_all(&state.pool)
            .await?;

        uploads_per_day.reverse();

        let mut purchase_by_day = sqlx::query_as!(DayCount,
            "SELECT COUNT(*) as count, DATE_TRUNC('day', purchase_time) as day from purchase GROUP BY DATE_TRUNC('day', purchase_time) ORDER BY day DESC LIMIT 15")
            .fetch_all(&state.pool)
            .await?;

        purchase_by_day.reverse();

        Ok::<_, sqlx::Error>(Stats {
            total_uploads: uploads.count.unwrap_or(0),
            active_listings: active_listings.count.unwrap_or(0),
            total_purchases: purchases.count.unwrap_or(0),
            unique_uploaders: unique_uploaders.count.unwrap_or(0),
            unique_items: unique_items.count.unwrap_or(0),
            uploads_per_day,
            purchase_by_day,
        })
    }).await?;

    Ok(Json(stats_value))
}

#[derive(Debug, Serialize, Clone, Copy)]
pub struct CacheStats {
    pub stats_cache_entry_count: u64,
    pub item_listings_entry_count: u64,
    pub item_purchase_entry_count: u64,
}

#[allow(clippy::unused_async)]
pub async fn cache_stats(State(state): State<AppState>) -> Result<Json<CacheStats>, AppError> {
    Ok(Json(CacheStats {
        stats_cache_entry_count: state.stats_cache.entry_count(),
        item_listings_entry_count: state.item_listings_cache.entry_count(),
        item_purchase_entry_count: state.item_purchase_cache.entry_count(),
    }))
}
