use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::TimeZone;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct UploadRequest<T> {
    pub world_id: u32,
    pub item_id: u32,
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
    pub price_per_unit: u32,
    pub quantity: u32,
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
    pub price_per_unit: u32,
    pub quantity: u32,
}

#[derive(Debug, Clone)]
struct AppState {
    pool: SqlitePool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();

    // initialize tracing
    tracing_subscriber::fmt::init();

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let state = AppState { pool };

    // build our application with a route
    let app = Router::new()
        .route("/history", post(upload_history))
        .route("/upload", post(upload))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn upload(
    State(state): State<AppState>,
    Json(payload): Json<UploadRequest<UploadRequestListing>>,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let date = chrono::Utc::now();

    sqlx::query!(
        "INSERT INTO upload (id, uploader_id, upload_time, world_id, item_id, upload_type)
        VALUES (?,?,?,?,?,?)",
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
    sqlx::query!("DELETE FROM listing WHERE item_id = ?", payload.item_id)
        .execute(&state.pool)
        .await?;

    for listing in payload.listings {
        let internal_id = Uuid::new_v4().to_string();
        let date = chrono::Utc
            .timestamp_opt(listing.last_review_time, 0)
            .unwrap();
        let materia_count = listing.materia.len() as i32;
        sqlx::query!(
            "INSERT INTO listing (
                id, upload_id, world_id, item_id, listing_id, seller_id,
                retainer_id, retainer_name, creator_id, creator_name,
                last_review_time, price_per_unit, quantity,
                retainer_city_id, materia_count, hq)
            VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            ",
            internal_id,
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
    let id = Uuid::new_v4().to_string();
    let date = chrono::Utc::now();

    sqlx::query!(
        "INSERT INTO upload (id, uploader_id, upload_time, world_id, item_id, upload_type)
        VALUES (?,?,?,?,?,?)",
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
         // delete records more recent than the last purchase time

        let oldest_purchase = payload
            .listings
            .iter()
            .map(|x| x.purchase_time)
            .min()
            .unwrap();
        let oldest_date = chrono::Utc.timestamp_opt(oldest_purchase, 0).unwrap();

        let trans = state.pool.begin().await?;

        sqlx::query!(
            "DELETE FROM history_listing WHERE item_id = ? AND purchase_time >= ?",
            payload.item_id,
            oldest_date
        )
        .execute(&state.pool)
        .await?;

        for listing in payload.listings {
            let date = chrono::Utc.timestamp_opt(listing.purchase_time, 0).unwrap();

            sqlx::query!(
                "INSERT INTO history_listing (
                    upload_id, item_id, world_id, buyer_name, hq, on_mannequin, purchase_time, quantity, price_per_unit)
                VALUES (?,?,?,?,?,?,?,?,?)
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
