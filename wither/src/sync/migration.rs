#![cfg_attr(feature="docinclude", doc(include="../docs/migrations-overview.md"))]

use mongodb::bson::{Bson, Document, doc};
use mongodb::options;
use mongodb::sync::{Collection, Database};

use crate::error::{Result, WitherError};
use crate::sync::ModelSync;

/// A trait describing a `Model` which has associated migrations.
pub trait MigratingSync: ModelSync {
    /// All migrations associated with this model.
    fn migrations() -> Vec<Box<dyn MigrationSync>>;

    /// Execute all migrations for this model.
    fn migrate(db: Database) -> Result<()> {
        let coll = Self::collection(db);
        let ns = coll.namespace();
        let migrations = Self::migrations();

        // Execute each migration.
        log::info!("Starting migrations for '{}'.", ns);
        for migration in migrations {
            migration.execute(&coll)?;
        }

        log::info!("Finished migrations for '{}'.", ns);
        Ok(())
    }
}

/// A trait describing objects which encapsulate a schema migration.
pub trait MigrationSync {
    /// The function which is to execute this migration.
    fn execute<'c>(&self, coll: &'c Collection) -> Result<()>;
}

/// A migration type which allows execution until the specifed `threshold` date. Then will no-op.
///
/// This migration type works nicely in environments where multiple instances of the system — in
/// which this migration is defined — are continuously running, even during deployment cycles.
/// AKA, highly available systems. With an `IntervalMigration`, each instance will execute the
/// migration at boottime, until the `threshold` date is passed. This will compensate for
/// write-heavy workloads, as the final instance to be updated will ensure schema convergence.
/// As long as you ensure your migrations are idempotent — **WHICH YOU ALWAYS SHOULD** — this
/// will work quite nicely.
pub struct IntervalMigrationSync {
    /// The name for this migration. Must be unique per collection.
    pub name: String,
    /// The UTC datetime when this migration should no longer execute.
    ///
    /// Use something like: `chrono::Utc.ymd(2017, 11, 20).and_hms(22, 37, 34)`.
    pub threshold: chrono::DateTime<chrono::Utc>,
    /// The filter to be used for selecting the documents to update.
    pub filter: Document,
    /// The document to be used for the `$set` operation of the update.
    pub set: Option<Document>,
    /// The document to be used for the `$unset` operation of the update.
    pub unset: Option<Document>,
}

impl MigrationSync for IntervalMigrationSync {
    fn execute<'c>(&self, coll: &'c Collection) -> Result<()> {
        let ns = coll.namespace();
        log::info!("Executing migration '{}' against '{}'.", &self.name, ns);

        // If the migrations threshold has been passed, then no-op.
        if chrono::Utc::now() > self.threshold {
            log::info!("Successfully executed migration '{}' against '{}'. No-op.", &self.name, ns);
            return Ok(());
        };

        // Build update document.
        let mut update = doc!{};
        if self.set.clone().is_none() && self.unset.clone().is_none() {
            return Err(WitherError::MigrationSetOrUnsetRequired);
        };
        if let Some(set) = self.set.clone() {
            update.insert("$set", Bson::from(set));
        }
        if let Some(unset) = self.unset.clone() {
            update.insert("$unset", Bson::from(unset));
        }

        // Build up & execute the migration.
        let options = options::UpdateOptions::builder()
            .upsert(Some(false))
            .write_concern(Some(options::WriteConcern::builder()
                .w(Some(options::Acknowledgment::Majority))
                .journal(Some(true))
                .build()
            ))
            .build();
        let res = coll.update_many(self.filter.clone(), update, Some(options))?;
        log::info!("Successfully executed migration '{}' against '{}'. {} matched. {} modified.", &self.name, ns, res.matched_count, res.modified_count);
        Ok(())
    }
}
