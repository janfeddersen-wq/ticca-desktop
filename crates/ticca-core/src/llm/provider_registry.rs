//! Provider registry and model routing

use crate::llm::providers::chatgpt::is_gpt_model;
use crate::llm::providers::gemini::is_gemini_model;
use crate::config::models::providers;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    Claude,
    Gemini,
    ChatGpt,
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::Claude => providers::CLAUDE,
            ProviderId::Gemini => providers::GEMINI,
            ProviderId::ChatGpt => providers::CHATGPT,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderId::Claude => "Claude",
            ProviderId::Gemini => "Gemini",
            ProviderId::ChatGpt => "ChatGPT",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub supports_tools: bool,
    pub supports_images: bool,
    pub supports_reasoning: bool,
    pub supports_streaming: bool,
    pub requires_id_token: bool,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub capabilities: ProviderCapabilities,
}

pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn resolve_provider(model_name: &str) -> ProviderId {
        if is_gpt_model(model_name) {
            ProviderId::ChatGpt
        } else if is_gemini_model(model_name) {
            ProviderId::Gemini
        } else {
            ProviderId::Claude
        }
    }

    pub fn info(provider: ProviderId) -> ProviderInfo {
        match provider {
            ProviderId::Claude => ProviderInfo {
                id: ProviderId::Claude,
                display_name: "Claude",
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_images: true,
                    supports_reasoning: true,
                    supports_streaming: true,
                    requires_id_token: false,
                },
            },
            ProviderId::Gemini => ProviderInfo {
                id: ProviderId::Gemini,
                display_name: "Gemini",
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_images: true,
                    supports_reasoning: true,
                    supports_streaming: true,
                    requires_id_token: false,
                },
            },
            ProviderId::ChatGpt => ProviderInfo {
                id: ProviderId::ChatGpt,
                display_name: "ChatGPT",
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_images: true,
                    supports_reasoning: true,
                    supports_streaming: true,
                    requires_id_token: true,
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_chatgpt_models() {
        assert_eq!(
            ProviderRegistry::resolve_provider("gpt-4o"),
            ProviderId::ChatGpt
        );
        assert_eq!(
            ProviderRegistry::resolve_provider("chatgpt-4o-latest"),
            ProviderId::ChatGpt
        );
    }

    #[test]
    fn resolves_gemini_models() {
        assert_eq!(
            ProviderRegistry::resolve_provider("gemini-2.0-flash"),
            ProviderId::Gemini
        );
    }

    #[test]
    fn resolves_claude_as_default() {
        assert_eq!(
            ProviderRegistry::resolve_provider("claude-3-5-sonnet"),
            ProviderId::Claude
        );
        assert_eq!(
            ProviderRegistry::resolve_provider("unknown-model"),
            ProviderId::Claude
        );
    }
}
