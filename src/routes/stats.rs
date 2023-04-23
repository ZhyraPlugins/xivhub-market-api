use std::{sync::Arc, time::Instant};

use crate::error::AppError;
use axum::{extract::State, Json};
use axum_prometheus::metrics::histogram;
use chrono::NaiveDate;
use color_eyre::eyre::eyre;
use serde::Serialize;
use tokio::{
    task::{JoinError, JoinHandle},
    try_join,
};

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
    pub day: Option<NaiveDate>,
}

#[derive(Debug, thiserror::Error)]
pub enum FlattenError {
    #[error("db error: {0}")]
    Db(#[from] Arc<sqlx::Error>),
    #[error("join error: {0}")]
    JoinHandle(#[from] JoinError),
}

async fn flatten<T: Send>(handle: JoinHandle<Result<T, sqlx::Error>>) -> Result<T, AppError> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(AppError(eyre!("db error: {:?}", err))),
        Err(err) => Err(AppError(eyre!("join error: {:?}", err))),
    }
}

pub async fn stats(State(state): State<AppState>) -> Result<Json<Stats>, AppError> {
    let stats_value = state.stats_cache.try_get_with((), async {

    let pool = state.pool.clone();
    let uploads = tokio::spawn(async move {
        let start = Instant::now();
        let q = sqlx::query!("SELECT COUNT(*) from upload")
            .fetch_one(&pool).await;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "stats_upload_count");
        q
    });

    let pool = state.pool.clone();
    let active_listings = tokio::spawn(async move {
        let start = Instant::now();
        let q = sqlx::query!("SELECT COUNT(*) from listing")
          .fetch_one(&pool).await;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "stats_listing_count");
        q
    });

    let pool = state.pool.clone();
    let purchases = tokio::spawn(async move {
        let start = Instant::now();
        let q = sqlx::query!("SELECT COUNT(*) from purchase")
            .fetch_one(&pool).await;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "stats_purchase_count");
        q
    });

    let pool = state.pool.clone();
    let unique_uploaders = tokio::spawn(async move {
        let start = Instant::now();
        let q = sqlx::query!("SELECT count FROM uploader_count")
            .fetch_one(&pool).await;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "stats_uploader_count");
        q
    });

    let pool = state.pool.clone();
    let unique_items = tokio::spawn(async move {
        let start = Instant::now();
        let q = sqlx::query!("SELECT count from unique_items_count")
            .fetch_one(&pool).await;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "stats_unique_items_count");
        q
    });

    let pool = state.pool.clone();
    let uploads_per_day = tokio::spawn(async move {
        let start = Instant::now();
        let q = sqlx::query_as!(DayCount,
            "SELECT COUNT(*) as count, date(timezone('UTC', upload_time)) as day from upload GROUP BY date(timezone('UTC', upload_time)) ORDER BY day DESC LIMIT 15")
            .fetch_all(&pool).await;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "stats_uploads_per_day_count");
        q
    });

    let pool = state.pool.clone();
    let purchase_by_day = tokio::spawn(async move {
        let start = Instant::now();
        let q = sqlx::query_as!(DayCount,
            "SELECT COUNT(*) as count, date(timezone('UTC', purchase_time)) as day from purchase GROUP BY date(timezone('UTC', purchase_time)) ORDER BY day DESC LIMIT 15")
            .fetch_all(&pool).await;
        let elapsed = start.elapsed();
        histogram!("xivhub_query", elapsed, "type" => "stats_purchases_by_day_count");
        q
    });

    let (uploads, active_listings, purchases, unique_uploaders, unique_items, mut uploads_per_day, mut purchase_by_day) = try_join!(
        flatten(uploads), flatten(active_listings), flatten(purchases),
        flatten(unique_uploaders), flatten(unique_items), flatten(uploads_per_day), flatten(purchase_by_day)
    )?;

    uploads_per_day.reverse();
    purchase_by_day.reverse();

    Ok::<_, AppError>(Stats {
        total_uploads: uploads.count.unwrap_or(0),
        active_listings: active_listings.count.unwrap_or(0),
        total_purchases: purchases.count.unwrap_or(0),
        unique_uploaders: unique_uploaders.count.unwrap_or(0),
        unique_items: unique_items.count.unwrap_or(0),
        uploads_per_day,
        purchase_by_day,
        })
    }).await.map_err(|e| eyre!("{:?}", e))?;

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
