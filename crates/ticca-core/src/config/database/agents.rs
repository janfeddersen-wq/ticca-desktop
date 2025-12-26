//! Agent model pinning CRUD operations.

use std::collections::HashMap;

use anyhow::Result;

use super::ConfigDatabase;

impl ConfigDatabase {
    /// Get the pinned model for a specific agent.
    pub fn get_agent_pinned_model(&self, agent_type: &str) -> Result<Option<String>> {
        let key = format!("agent_pinned_model.{}", agent_type);
        match self.get_setting(&key)? {
            Some(setting) if !setting.value.is_empty() => Ok(Some(setting.value)),
            _ => Ok(None),
        }
    }

    /// Set the pinned model for a specific agent.
    pub fn set_agent_pinned_model(&self, agent_type: &str, model_name: &str) -> Result<()> {
        let key = format!("agent_pinned_model.{}", agent_type);
        self.set_setting(&key, model_name)
    }

    /// Clear the pinned model for a specific agent.
    pub fn clear_agent_pinned_model(&self, agent_type: &str) -> Result<()> {
        let key = format!("agent_pinned_model.{}", agent_type);
        self.delete_setting(&key)?;
        Ok(())
    }

    /// Get all agent-model pinnings.
    pub fn get_all_agent_pinned_models(&self) -> Result<HashMap<String, String>> {
        let settings = self.get_all_settings()?;
        let mut pinnings = HashMap::new();

        for setting in settings {
            if let Some(agent_type) = setting.key.strip_prefix("agent_pinned_model.")
                && !setting.value.is_empty()
            {
                pinnings.insert(agent_type.to_string(), setting.value);
            }
        }

        Ok(pinnings)
    }
}
