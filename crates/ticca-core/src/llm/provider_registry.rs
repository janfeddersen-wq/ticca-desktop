//! Provider registry and model routing

use crate::config::models::providers;
use crate::config::ApiKeyProvider;
use crate::llm::providers::chatgpt::is_gpt_model;
use crate::llm::providers::gemini::is_gemini_model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    Claude,
    Gemini,
    ChatGpt,
    ApiKey(ApiKeyProvider),
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderId::Claude => providers::CLAUDE,
            ProviderId::Gemini => providers::GEMINI,
            ProviderId::ChatGpt => providers::CHATGPT,
            ProviderId::ApiKey(p) => p.id(),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderId::Claude => "Claude",
            ProviderId::Gemini => "Gemini",
            ProviderId::ChatGpt => "ChatGPT",
            ProviderId::ApiKey(p) => p.display_name(),
        }
    }

    pub fn is_oauth(&self) -> bool {
        matches!(self, ProviderId::Claude | ProviderId::Gemini | ProviderId::ChatGpt)
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
    /// Extract the actual model ID from a display name like "model-id - Provider Name"
    pub fn extract_model_id(model_name: &str) -> &str {
        if let Some(idx) = model_name.rfind(" - ") {
            model_name[..idx].trim()
        } else {
            model_name
        }
    }

    /// Extract the provider suffix from a display name like "model-id - Provider Name"
    fn extract_provider_suffix(model_name: &str) -> Option<&str> {
        if let Some(idx) = model_name.rfind(" - ") {
            Some(model_name[idx + 3..].trim())
        } else {
            None
        }
    }

    pub fn resolve_provider(model_name: &str) -> ProviderId {
        // First check if the model name has a provider suffix
        if let Some(suffix) = Self::extract_provider_suffix(model_name) {
            // Check OAuth providers
            if suffix.contains("Claude") && suffix.contains("OAuth") {
                return ProviderId::Claude;
            }
            if suffix.contains("Gemini") && suffix.contains("OAuth") {
                return ProviderId::Gemini;
            }
            if suffix.contains("ChatGPT") && suffix.contains("OAuth") {
                return ProviderId::ChatGpt;
            }

            // Check API key providers by display name
            for provider in ApiKeyProvider::ALL {
                if suffix == provider.display_name() {
                    return ProviderId::ApiKey(*provider);
                }
            }
        }

        // Fall back to model name pattern matching (for backwards compatibility)
        let model_id = Self::extract_model_id(model_name);
        if is_gpt_model(model_id) {
            ProviderId::ChatGpt
        } else if is_gemini_model(model_id) {
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
            ProviderId::ApiKey(api_provider) => ProviderInfo {
                id: ProviderId::ApiKey(api_provider),
                display_name: api_provider.display_name(),
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_images: api_provider.is_openai_compatible(),
                    supports_reasoning: true,
                    supports_streaming: true,
                    requires_id_token: false,
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
