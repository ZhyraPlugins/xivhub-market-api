#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]

use moka::future::Cache;
use routes::{
    item::{ListingsResponse, PurchasesResponse},
    stats::Stats,
};
pub use sqlx::PgPool;

pub mod entities;
pub mod error;
pub mod routes;
pub mod util;

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: PgPool,
    // Since stats is just 1 object, we make a simple cache.
    pub stats_cache: Cache<(), Stats>,
    pub item_listings_cache: Cache<i32, ListingsResponse>,
    // For now, only page 0 is cached. The only page used by the current frontend.
    pub item_purchase_cache: Cache<i32, PurchasesResponse>,
}
