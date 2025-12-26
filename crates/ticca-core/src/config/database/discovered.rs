//! Discovered models CRUD operations (model registry cache).

use anyhow::Result;
use rusqlite::params;

use crate::config::models::DiscoveredModel;

use super::ConfigDatabase;

impl ConfigDatabase {
    /// Get a discovered model by its canonical ID.
    pub fn get_discovered_model(&self, canonical_id: &str) -> Result<Option<DiscoveredModel>> {
        let mut stmt = self.conn.prepare(
            "SELECT canonical_id, provider, model_id, display_name, context_length, discovered_at
             FROM discovered_models WHERE canonical_id = ?",
        )?;

        let result = stmt.query_row(params![canonical_id], |row| {
            Ok(DiscoveredModel {
                canonical_id: row.get(0)?,
                provider: row.get(1)?,
                model_id: row.get(2)?,
                display_name: row.get(3)?,
                context_length: row.get(4)?,
                discovered_at: row.get(5)?,
            })
        });

        match result {
            Ok(model) => Ok(Some(model)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all discovered models, optionally filtered by provider.
    pub fn list_discovered_models(&self, provider: Option<&str>) -> Result<Vec<DiscoveredModel>> {
        fn map_model(row: &rusqlite::Row<'_>) -> rusqlite::Result<DiscoveredModel> {
            Ok(DiscoveredModel {
                canonical_id: row.get(0)?,
                provider: row.get(1)?,
                model_id: row.get(2)?,
                display_name: row.get(3)?,
                context_length: row.get(4)?,
                discovered_at: row.get(5)?,
            })
        }

        let mut stmt = if provider.is_some() {
            self.conn.prepare(
                "SELECT canonical_id, provider, model_id, display_name, context_length, discovered_at
                 FROM discovered_models WHERE provider = ? ORDER BY model_id",
            )?
        } else {
            self.conn.prepare(
                "SELECT canonical_id, provider, model_id, display_name, context_length, discovered_at
                 FROM discovered_models ORDER BY provider, model_id",
            )?
        };

        let rows = if let Some(p) = provider {
            stmt.query_map(params![p], map_model)?
        } else {
            stmt.query_map([], map_model)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Upsert a discovered model (insert or update).
    pub fn upsert_discovered_model(&self, model: &DiscoveredModel) -> Result<()> {
        self.conn.execute(
            "INSERT INTO discovered_models (canonical_id, provider, model_id, display_name, context_length, discovered_at)
             VALUES (?, ?, ?, ?, ?, COALESCE(?, datetime('now')))
             ON CONFLICT(canonical_id) DO UPDATE SET
                provider = excluded.provider,
                model_id = excluded.model_id,
                display_name = excluded.display_name,
                context_length = excluded.context_length,
                discovered_at = excluded.discovered_at",
            params![
                model.canonical_id,
                model.provider,
                model.model_id,
                model.display_name,
                model.context_length,
                model.discovered_at,
            ],
        )?;
        Ok(())
    }

    /// Batch upsert discovered models.
    pub fn upsert_discovered_models_batch(&self, models: &[DiscoveredModel]) -> Result<()> {
        for model in models {
            self.upsert_discovered_model(model)?;
        }
        Ok(())
    }

    /// Delete all discovered models for a provider.
    pub fn delete_discovered_models_for_provider(&self, provider: &str) -> Result<usize> {
        let changes = self.conn.execute(
            "DELETE FROM discovered_models WHERE provider = ?",
            params![provider],
        )?;
        Ok(changes)
    }

    /// Delete a specific discovered model by canonical ID.
    pub fn delete_discovered_model(&self, canonical_id: &str) -> Result<bool> {
        let changes = self.conn.execute(
            "DELETE FROM discovered_models WHERE canonical_id = ?",
            params![canonical_id],
        )?;
        Ok(changes > 0)
    }

    /// Get the context length for a model by its canonical ID.
    pub fn get_model_context_length(&self, canonical_id: &str) -> Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT context_length FROM discovered_models WHERE canonical_id = ?")?;

        let result = stmt.query_row(params![canonical_id], |row| row.get(0));

        match result {
            Ok(length) => Ok(Some(length)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
