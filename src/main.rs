#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]

use axum::{
    routing::{get, post},
    Router,
};
use axum_prometheus::PrometheusMetricLayer;
use entities::ItemInfo;
use error::AppError;
use headers::HeaderValue;
use reqwest::{header, Method};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{net::SocketAddr, time::Duration};
use tower_http::{
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::info;

pub mod entities;
pub mod error;
pub mod routes;

#[derive(Debug, Clone)]
pub struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();

    // initialize tracing
    tracing_subscriber::fmt::init();

    let (prometheus_layer, metrics_handle) = PrometheusMetricLayer::pair();

    let pool = PgPoolOptions::new()
        .max_connections(
            std::env::var("DATABASE_MAX_CONNECTIONS")
                .map(|x| x.parse().expect("valid number"))
                .unwrap_or(30),
        )
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let state = AppState { pool: pool.clone() };

    tokio::spawn(async move {
        info!("Started item info fetcher");

        let current_item_ids = sqlx::query!("SELECT DISTINCT item_id from upload")
            .fetch_all(&pool)
            .await
            .unwrap();

        let total = current_item_ids.len();

        for (i, record) in current_item_ids.into_iter().enumerate() {
            let item_info = sqlx::query_as!(
                ItemInfo,
                "SELECT * from item_info WHERE item_id = $1",
                record.item_id
            )
            .fetch_optional(&pool)
            .await
            .unwrap();

            info!("Fetching item infos. ({}/{})", i + 1, total);
            if item_info.is_none() {
                info!("Item info with id {} not in db, fetching.", record.item_id);
                fetch_item_info(record.item_id, &pool).await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    });

    // build our application with a route
    let app = Router::new()
        .route("/", get(|| async { include_str!("../README.md") }))
        .route("/last_uploads", get(routes::upload::last_uploads))
        .route("/history", post(routes::upload::history))
        .route("/upload", post(routes::upload::listings))
        .route(
            "/stats",
            get(routes::stats::stats).layer(SetResponseHeaderLayer::if_not_present(
                header::CACHE_CONTROL,
                HeaderValue::from_static("max-age=300, must-revalidate"),
            )),
        )
        .route("/item", get(routes::item::list))
        .route("/item/:id", get(routes::item::listings))
        .route("/item/:id/purchases", get(routes::item::purchases))
        .route(
            "/item/:id/uploads",
            get(routes::item::get_item_upload_dates),
        )
        .route("/metrics", get(|| async move { metrics_handle.render() }))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_origin(Any),
        )
        .layer(prometheus_layer)
        .with_state(state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct XivApiItemInfo {
    pub name: String,
    pub icon: String,
    #[serde(rename = "IconHD")]
    pub icon_hd: String,
    pub description: String,
    pub item_kind: ItemKind,
    pub item_search_category: SearchCategory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemKind {
    #[serde(rename = "ID")]
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SearchCategory {
    pub category: i32,
    #[serde(rename = "IconHD")]
    pub icon_hd: String,
    pub name: String,
}

/// Fetches the info into db if it doesnt exist yet.
async fn fetch_item_info(item_id: i32, db: &PgPool) -> Result<ItemInfo, AppError> {
    let res = sqlx::query_as!(
        ItemInfo,
        "SELECT * FROM item_info WHERE item_id = $1",
        item_id
    )
    .fetch_optional(db)
    .await?;

    if let Some(res) = res {
        Ok(res)
    } else {
        let private_key = std::env::var("XIVAPI_PRIVATE_KEY")?;
        let data = reqwest::get(format!("https://xivapi.com/item/{item_id}?private_key={private_key}&columns=Name,Icon,IconHD,Description,ItemKind.Name,ItemKind.ID,ItemSearchCategory.Category,ItemSearchCategory.IconHD,ItemSearchCategory.Name")).await?;
        let res: XivApiItemInfo = data.json().await?;
        info!("Fetched info for item {item_id}");

        sqlx::query!("INSERT INTO item_info (item_id, name, icon, icon_hd, description, item_kind_name, item_kind_id,
                        item_search_category, item_search_category_iconhd, item_search_category_name)
                        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)", 
                        item_id,
                    res.name,
                    res.icon,
                    res.icon_hd,
                    res.description,
                    res.item_kind.name,
                    res.item_kind.id,
                    res.item_search_category.category,
                    res.item_search_category.icon_hd,
                    res.item_search_category.name,
                )
                        .execute(db).await.ok();

        let res = sqlx::query_as!(
            ItemInfo,
            "SELECT * FROM item_info WHERE item_id = $1",
            item_id
        )
        .fetch_one(db)
        .await?;

        Ok(res)
    }
}
