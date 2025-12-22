//! Agent profile abstraction

use super::{AgentType, get_agent};
use crate::llm::{ProviderId, ProviderRegistry};

#[derive(Debug, Clone)]
pub struct ToolUsagePolicy {
    pub read_before_write: bool,
    pub prefer_edit_over_write: bool,
    pub max_file_lines: usize,
}

impl ToolUsagePolicy {
    pub fn coding() -> Self {
        Self {
            read_before_write: true,
            prefer_edit_over_write: true,
            max_file_lines: 600,
        }
    }

    pub fn planning() -> Self {
        Self {
            read_before_write: true,
            prefer_edit_over_write: false,
            max_file_lines: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccountSelectionPolicy {
    pub provider_order: Vec<ProviderId>,
    pub cooldown_aware: bool,
}

impl AccountSelectionPolicy {
    pub fn for_agent(agent_type: AgentType) -> Self {
        let provider_order = match agent_type {
            AgentType::Coding | AgentType::Skills => {
                vec![ProviderId::Claude, ProviderId::ChatGpt, ProviderId::Gemini]
            }
            AgentType::Planning => {
                vec![ProviderId::Claude, ProviderId::Gemini, ProviderId::ChatGpt]
            }
        };

        Self {
            provider_order,
            cooldown_aware: true,
        }
    }

    pub fn pick_provider<F>(&self, mut is_available: F) -> Option<ProviderId>
    where
        F: FnMut(ProviderId) -> bool,
    {
        self.provider_order
            .iter()
            .copied()
            .find(|provider| is_available(*provider))
    }
}

#[derive(Debug, Clone)]
pub enum ModelRule {
    Pinned,
    Default,
    ProviderFallback(Vec<ProviderId>),
    AnyAvailable,
}

#[derive(Debug, Clone)]
pub struct ModelStrategy {
    pub rules: Vec<ModelRule>,
}

impl ModelStrategy {
    pub fn new(rules: Vec<ModelRule>) -> Self {
        Self { rules }
    }

    pub fn resolve(&self, context: ModelSelectionContext<'_>) -> Option<String> {
        for rule in &self.rules {
            match rule {
                ModelRule::Pinned => {
                    if let Some(value) = context.pinned {
                        return Some(value.to_string());
                    }
                }
                ModelRule::Default => {
                    if let Some(value) = context.default_model {
                        return Some(value.to_string());
                    }
                }
                ModelRule::ProviderFallback(order) => {
                    if let Some(model) =
                        find_model_by_provider(context.available_models, order.as_slice())
                    {
                        return Some(model);
                    }
                }
                ModelRule::AnyAvailable => {
                    if let Some(model) = context.available_models.first() {
                        return Some(model.clone());
                    }
                }
            }
        }

        None
    }
}

fn find_model_by_provider(models: &[String], providers: &[ProviderId]) -> Option<String> {
    for provider in providers {
        if let Some(model) = models
            .iter()
            .find(|name| ProviderRegistry::resolve_provider(name) == *provider)
        {
            return Some(model.clone());
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
pub struct ModelSelectionContext<'a> {
    pub pinned: Option<&'a str>,
    pub default_model: Option<&'a str>,
    pub available_models: &'a [String],
}

#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub agent_type: AgentType,
    pub system_prompt: String,
    pub tool_names: Vec<&'static str>,
    pub max_tool_rounds: u32,
    pub tool_usage_policy: ToolUsagePolicy,
    pub account_policy: AccountSelectionPolicy,
    pub model_strategy: ModelStrategy,
}

impl AgentProfile {
    pub fn for_type(agent_type: AgentType, max_tool_rounds: u32) -> Self {
        let agent = get_agent(agent_type);
        let tool_usage_policy = match agent_type {
            AgentType::Coding | AgentType::Skills => ToolUsagePolicy::coding(),
            AgentType::Planning => ToolUsagePolicy::planning(),
        };
        let account_policy = AccountSelectionPolicy::for_agent(agent_type);
        let model_strategy = ModelStrategy::new(vec![
            ModelRule::Pinned,
            ModelRule::Default,
            ModelRule::ProviderFallback(account_policy.provider_order.clone()),
            ModelRule::AnyAvailable,
        ]);

        Self {
            agent_type,
            system_prompt: agent.system_prompt(),
            tool_names: agent.available_tools(),
            max_tool_rounds,
            tool_usage_policy,
            account_policy,
            model_strategy,
        }
    }

    pub fn resolve_model(&self, context: ModelSelectionContext<'_>) -> Option<String> {
        self.model_strategy.resolve(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_strategy_prefers_pinned_then_default() {
        let strategy = ModelStrategy::new(vec![ModelRule::Pinned, ModelRule::Default]);
        let models = vec!["claude-test".to_string()];
        let context = ModelSelectionContext {
            pinned: Some("pinned-model"),
            default_model: Some("default-model"),
            available_models: &models,
        };

        assert_eq!(strategy.resolve(context), Some("pinned-model".to_string()));
    }

    #[test]
    fn model_strategy_falls_back_to_provider_order() {
        let models = vec![
            "gpt-4o".to_string(),
            "claude-3-sonnet".to_string(),
            "gemini-1.5-pro".to_string(),
        ];
        let strategy = ModelStrategy::new(vec![ModelRule::ProviderFallback(vec![
            ProviderId::Claude,
            ProviderId::ChatGpt,
        ])]);
        let context = ModelSelectionContext {
            pinned: None,
            default_model: None,
            available_models: &models,
        };

        assert_eq!(
            strategy.resolve(context),
            Some("claude-3-sonnet".to_string())
        );
    }

    #[test]
    fn account_policy_prefers_first_available() {
        let policy = AccountSelectionPolicy::for_agent(AgentType::Coding);
        let picked = policy.pick_provider(|provider| provider == ProviderId::ChatGpt);
        assert_eq!(picked, Some(ProviderId::ChatGpt));
    }
}
