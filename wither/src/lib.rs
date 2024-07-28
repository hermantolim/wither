#![cfg_attr(feature = "docinclude", doc = include_str!("../README.md"))]

// Re-exports //
pub use async_trait::async_trait;
pub use mongodb;
pub use mongodb::bson;

pub use common::IndexModel;
pub use cursor::ModelCursor;
pub use error::{Result, WitherError};
pub use migration::{IntervalMigration, Migration};
pub use model::Model;
pub use wither_derive::Model;

// Common //
mod common;
mod error;
// Async //
mod cursor;
mod migration;
mod model;

/// All traits needed for basic usage of the wither system.
pub mod prelude {
    pub use wither_derive::Model;

    pub use crate::migration::{Migrating, Migration};
    pub use crate::model::Model;
}
