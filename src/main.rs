use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::TimeZone;
use entities::Purchase;
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::SocketAddr, time::Duration};
use tower_http::{trace::TraceLayer, timeout::{Timeout, TimeoutLayer}};
use tracing::info;
use uuid::Uuid;

use crate::entities::Listing;

mod entities;

#[derive(Debug, Deserialize)]
struct UploadRequest<T> {
    pub world_id: i32,
    pub item_id: i32,
    pub uploader_id: String,
    pub listings: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct UploadRequestListing {
    pub listing_id: i64,
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

#[derive(Debug, Deserialize)]
struct ItemMateria {
    pub slot_id: i32,
    pub materia_id: i32,
}

#[derive(Debug, Deserialize)]
struct UploadHistoryRequestListing {
    pub hq: bool,
    pub buyer_name: String,
    pub on_mannequin: bool,
    pub purchase_time: i64,
    pub price_per_unit: i32,
    pub quantity: i32,
}

#[derive(Debug, Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();

    // initialize tracing
    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(30)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let state = AppState { pool };

    // build our application with a route
    let app = Router::new()
        .route("/history", post(upload_history))
        .route("/upload", post(upload))
        .route("/item/:id", get(get_item_listings))
        .route("/item/:id/purchases", get(get_item_purchases))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn upload(
    State(state): State<AppState>,
    Json(payload): Json<UploadRequest<UploadRequestListing>>,
) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    let date = chrono::Utc::now();
    info!("Received upload for item {}", payload.item_id);

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
    sqlx::query!(
        "DELETE FROM listing WHERE item_id = $1 AND world_id =$2",
        payload.item_id,
        payload.world_id
    )
    .execute(&state.pool)
    .await?;

    for listing in payload.listings {
        let date = chrono::Utc
            .timestamp_opt(listing.last_review_time, 0)
            .unwrap();
        let materia_count = listing.materia.len() as i32;
        sqlx::query!(
            "INSERT INTO listing (
                upload_id, world_id, item_id, listing_id, seller_id,
                retainer_id, retainer_name, creator_id, creator_name,
                last_review_time, price_per_unit, quantity,
                retainer_city_id, materia_count, hq)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            ",
            id,
            payload.world_id,
            payload.item_id,
            listing.listing_id,
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
        .await?;
    }

    Ok(())
}

async fn upload_history(
    State(state): State<AppState>,
    Json(payload): Json<UploadRequest<UploadHistoryRequestListing>>,
) -> Result<(), AppError> {
    let id = Uuid::new_v4();
    let date = chrono::Utc::now();
    info!(
        "Received purchase history upload for item {}",
        payload.item_id
    );

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
    .execute(&state.pool)
    .await?;

    if !payload.listings.is_empty() {
        let oldest_purchase = payload
            .listings
            .iter()
            .map(|x| x.purchase_time)
            .min()
            .unwrap();
        let oldest_date = chrono::Utc.timestamp_opt(oldest_purchase, 0).unwrap();

        let trans = state.pool.begin().await?;

        // delete records more recent than the last purchase time
        sqlx::query!(
            "DELETE FROM purchase WHERE item_id = $1 AND world_id = $2 AND purchase_time >= $3",
            payload.item_id,
            payload.world_id,
            oldest_date
        )
        .execute(&state.pool)
        .await?;

        for listing in payload.listings {
            let date = chrono::Utc.timestamp_opt(listing.purchase_time, 0).unwrap();

            sqlx::query!(
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
            .execute(&state.pool)
            .await?;
        }

        trans.commit().await?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ListingsQuery {}

async fn get_item_listings(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
) -> Result<Json<Vec<Listing>>, AppError> {
    let listings = sqlx::query_as!(
        Listing,
        "SELECT * FROM listing WHERE item_id = $1 ORDER BY world_id ASC, price_per_unit ASC",
        item_id
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(listings))
}

#[derive(Debug, Deserialize)]
struct PurchasesQuery {
    pub page: Option<i64>,
}

async fn get_item_purchases(
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Query(query): Query<PurchasesQuery>
) -> Result<Json<Vec<Purchase>>, AppError> {
    let page = query.page.unwrap_or(0);

    let listings = sqlx::query_as!(
        Purchase,
        "SELECT * FROM purchase WHERE item_id = $1 ORDER BY world_id ASC, purchase_time DESC OFFSET $2 LIMIT $3",
        item_id,
        page * 250,
        250
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(listings))
}

/*

    /// <summary>
    /// Upload data about an item.
    /// </summary>
    /// <param name="item">The item request data being uploaded.</param>
    /// <returns>An async task.</returns>
    Task Upload(MarketBoardItemRequest item);

    /// <summary>
    /// Upload tax rate data.
    /// </summary>
    /// <param name="taxRates">The tax rate data being uploaded.</param>
    /// <returns>An async task.</returns>
    Task UploadTax(MarketTaxRates taxRates);

    /// <summary>
    /// Upload information about a purchase this client has made.
    /// </summary>
    /// <param name="purchaseHandler">The purchase handler data associated with the sale.</param>
    /// <returns>An async task.</returns>
    Task UploadPurchase(MarketBoardPurchaseHandler purchaseHandler);
*/

struct AppError(color_eyre::eyre::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<color_eyre::eyre::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
