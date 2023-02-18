#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]

pub use sqlx::PgPool;

pub mod entities;
pub mod error;
pub mod routes;
pub mod util;

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: PgPool,
}
