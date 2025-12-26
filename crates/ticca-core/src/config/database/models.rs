//! Model configuration CRUD operations.

use anyhow::Result;
use rusqlite::params;

use crate::config::models::ModelConfig;

use super::ConfigDatabase;

impl ConfigDatabase {
    /// Get a model configuration by ID.
    pub fn get_model(&self, id: &str) -> Result<Option<ModelConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at
             FROM models WHERE id = ?",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(ModelConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                model_type: row.get(2)?,
                endpoint_url: row.get(3)?,
                context_length: row.get(4)?,
                is_default: row.get(5)?,
                config_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        });

        match result {
            Ok(model) => Ok(Some(model)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all model configurations.
    pub fn get_all_models(&self) -> Result<Vec<ModelConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at
             FROM models ORDER BY name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ModelConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                model_type: row.get(2)?,
                endpoint_url: row.get(3)?,
                context_length: row.get(4)?,
                is_default: row.get(5)?,
                config_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Get the default model configuration.
    pub fn get_default_model(&self) -> Result<Option<ModelConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at
             FROM models WHERE is_default = 1 LIMIT 1",
        )?;

        let result = stmt.query_row([], |row| {
            Ok(ModelConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                model_type: row.get(2)?,
                endpoint_url: row.get(3)?,
                context_length: row.get(4)?,
                is_default: row.get(5)?,
                config_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        });

        match result {
            Ok(model) => Ok(Some(model)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Upsert a model configuration.
    pub fn upsert_model(&self, model: &ModelConfig) -> Result<()> {
        // If this model is default, clear other defaults first
        if model.is_default {
            self.conn.execute("UPDATE models SET is_default = 0", [])?;
        }

        self.conn.execute(
            "INSERT INTO models (id, name, model_type, endpoint_url, context_length, is_default, config_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, COALESCE(?, datetime('now')))
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                model_type = excluded.model_type,
                endpoint_url = excluded.endpoint_url,
                context_length = excluded.context_length,
                is_default = excluded.is_default,
                config_json = excluded.config_json",
            params![
                model.id,
                model.name,
                model.model_type,
                model.endpoint_url,
                model.context_length,
                model.is_default,
                model.config_json,
                model.created_at,
            ],
        )?;
        Ok(())
    }

    /// Delete a model configuration by ID.
    pub fn delete_model(&self, id: &str) -> Result<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM models WHERE id = ?", params![id])?;
        Ok(changes > 0)
    }
}
