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
use headers::HeaderValue;
use reqwest::{header, Method};
use sqlx::postgres::PgPoolOptions;
use std::{net::SocketAddr, time::Duration};
use tower_http::{
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use xivhub_market::{routes, AppState};

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

    let state = AppState { pool: pool.clone() };

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
