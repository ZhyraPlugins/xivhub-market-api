//! The api server.

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
use moka::future::Cache;
use reqwest::Method;
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::error;
use xivhub_market::{
    routes::{self},
    AppState,
};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Better error reporting whe compiling if we are not inside a macro.
    run().await
}

async fn run() -> color_eyre::Result<()> {
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

    let items_cache_capacity = std::env::var("XIVHUB_ITEMS_CACHE_SIZE")
        .map(|x| x.parse().expect("valid number"))
        .unwrap_or(5000);

    let state = AppState {
        pool: pool.clone(),
        item_listings_cache: Cache::builder()
            .name("item_listings_cache")
            .time_to_idle(Duration::from_secs(60 * 60 * 6)) // 6 hours, updates invalidate the entry
            .max_capacity(items_cache_capacity)
            .build(),
        item_purchase_cache: Cache::builder()
            .name("item_purchase_cache")
            .time_to_idle(Duration::from_secs(60 * 60 * 6)) // 6 hours, updates invalidate the entry
            .max_capacity(items_cache_capacity)
            .build(),
        stats_cache: Cache::builder()
            .name("stats_cache")
            .time_to_live(Duration::from_secs(60 * 5)) // 5 minutes
            .max_capacity(1)
            .build(),
    };

    let sched = JobScheduler::new().await?;

    let sched_pool = pool.clone();
    sched
        .add(Job::new_repeated_async(
            Duration::from_secs(60 * 30),
            move |_, _sched| {
                let sched_pool = sched_pool.clone();
                Box::pin(async move {
                    let result = sqlx::query!(
                        "delete from purchase where purchase_time < NOW() - INTERVAL '1 months'"
                    )
                    .execute(&sched_pool)
                    .await;

                    if let Err(e) = result {
                        error!("task (sched) error: {}", e);
                    }
                })
            },
        )?)
        .await?;

    // build our application with a route
    let app = Router::new()
        .route("/", get(|| async { include_str!("../README.md") }))
        .route("/last_uploads", get(routes::upload::last_uploads))
        .route("/history", post(routes::upload::history))
        .route("/upload", post(routes::upload::listings))
        .route("/stats", get(routes::stats::stats))
        .route("/cache_stats", get(routes::stats::cache_stats))
        .route("/item", get(routes::item::list))
        .route("/item/:id", get(routes::item::listings))
        .route("/item/:id/purchases", get(routes::item::purchases))
        .route(
            "/item/:id/purchases_by_day",
            get(routes::item::purchases_by_day),
        )
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
