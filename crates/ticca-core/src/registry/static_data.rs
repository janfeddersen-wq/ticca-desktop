//! Static model registry data
//!
//! This module contains model definitions bundled at build time.
//! Eventually this will be generated from the models.dev API:
//! https://models.dev/api.json
//!
//! The data here serves as a fallback and ensures we have accurate
//! context window sizes even without network access.

use super::{AuthType, ModelCapabilities, ModelDefinition, ProviderDefinition};

/// Get all statically defined models.
///
/// These are the core models we know about at compile time.
/// Runtime discovery can supplement this list with newer models
/// or models from additional providers.
///
/// # Model Coverage
///
/// - **Anthropic**: Claude 4 (Opus, Sonnet), Claude 3.5 (Sonnet, Haiku)
/// - **OpenAI**: GPT-4o, GPT-4 Turbo, o1/o3 reasoning models
/// - **Google**: Gemini 2.0/2.5 Flash and Pro
/// - **Groq**: Fast inference models (Llama, Mixtral)
/// - **Mistral**: Mistral Large, Codestral
/// - **DeepSeek**: DeepSeek V3, Coder
/// - **Together**: Various open models
pub fn static_models() -> Vec<ModelDefinition> {
    vec![
        // ============================================
        // Anthropic Claude Models
        // ============================================
        ModelDefinition {
            id: "claude-sonnet-4-20250514".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude Sonnet 4".to_string(),
            family: Some("claude-4".to_string()),
            context_window: 200_000,
            max_output_tokens: Some(64_000),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: Some("2025-03".to_string()),
            release_date: Some("2025-05-14".to_string()),
        },
        ModelDefinition {
            id: "claude-opus-4-20250514".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude Opus 4".to_string(),
            family: Some("claude-4".to_string()),
            context_window: 200_000,
            max_output_tokens: Some(32_000),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: Some("2025-03".to_string()),
            release_date: Some("2025-05-14".to_string()),
        },
        ModelDefinition {
            id: "claude-3-5-sonnet-20241022".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            family: Some("claude-3.5".to_string()),
            context_window: 200_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: Some("2024-04".to_string()),
            release_date: Some("2024-10-22".to_string()),
        },
        ModelDefinition {
            id: "claude-3-5-haiku-20241022".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude 3.5 Haiku".to_string(),
            family: Some("claude-3.5".to_string()),
            context_window: 200_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: true,
                reasoning: false, // Haiku is faster, less reasoning
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: Some("2024-04".to_string()),
            release_date: Some("2024-10-22".to_string()),
        },
        ModelDefinition {
            id: "claude-3-opus-20240229".to_string(),
            provider_id: "anthropic".to_string(),
            name: "Claude 3 Opus".to_string(),
            family: Some("claude-3".to_string()),
            context_window: 200_000,
            max_output_tokens: Some(4_096),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: Some("2023-08".to_string()),
            release_date: Some("2024-02-29".to_string()),
        },
        // ============================================
        // OpenAI GPT Models
        // ============================================
        ModelDefinition {
            id: "gpt-4o".to_string(),
            provider_id: "openai".to_string(),
            name: "GPT-4o".to_string(),
            family: Some("gpt-4".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(16_384),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: Some("2024-10".to_string()),
            release_date: Some("2024-05-13".to_string()),
        },
        ModelDefinition {
            id: "gpt-4o-mini".to_string(),
            provider_id: "openai".to_string(),
            name: "GPT-4o Mini".to_string(),
            family: Some("gpt-4".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(16_384),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: Some("2024-10".to_string()),
            release_date: Some("2024-07-18".to_string()),
        },
        ModelDefinition {
            id: "gpt-4-turbo".to_string(),
            provider_id: "openai".to_string(),
            name: "GPT-4 Turbo".to_string(),
            family: Some("gpt-4".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(4_096),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: Some("2023-12".to_string()),
            release_date: Some("2024-04-09".to_string()),
        },
        ModelDefinition {
            id: "gpt-4".to_string(),
            provider_id: "openai".to_string(),
            name: "GPT-4".to_string(),
            family: Some("gpt-4".to_string()),
            context_window: 8_192,
            max_output_tokens: Some(4_096),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false, // Original GPT-4 didn't have vision
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: Some("2023-04".to_string()),
            release_date: Some("2023-03-14".to_string()),
        },
        // OpenAI Reasoning Models
        ModelDefinition {
            id: "o1-preview".to_string(),
            provider_id: "openai".to_string(),
            name: "o1 Preview".to_string(),
            family: Some("o1".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(32_768),
            capabilities: ModelCapabilities {
                tool_call: false, // o1 doesn't support tools yet
                vision: true,
                reasoning: true,
                streaming: false, // o1 doesn't stream
                json_mode: true,
            },
            knowledge_cutoff: Some("2023-10".to_string()),
            release_date: Some("2024-09-12".to_string()),
        },
        ModelDefinition {
            id: "o1-mini".to_string(),
            provider_id: "openai".to_string(),
            name: "o1 Mini".to_string(),
            family: Some("o1".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(65_536),
            capabilities: ModelCapabilities {
                tool_call: false,
                vision: false,
                reasoning: true,
                streaming: false,
                json_mode: true,
            },
            knowledge_cutoff: Some("2023-10".to_string()),
            release_date: Some("2024-09-12".to_string()),
        },
        ModelDefinition {
            id: "o3-mini".to_string(),
            provider_id: "openai".to_string(),
            name: "o3 Mini".to_string(),
            family: Some("o3".to_string()),
            context_window: 200_000,
            max_output_tokens: Some(100_000),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: true,
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: Some("2024-10".to_string()),
            release_date: Some("2025-01-31".to_string()),
        },
        // ============================================
        // Google Gemini Models
        // ============================================
        ModelDefinition {
            id: "gemini-2.0-flash".to_string(),
            provider_id: "google".to_string(),
            name: "Gemini 2.0 Flash".to_string(),
            family: Some("gemini-2".to_string()),
            context_window: 1_000_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: None,
            release_date: Some("2024-12".to_string()),
        },
        ModelDefinition {
            id: "gemini-2.5-flash-preview-05-20".to_string(),
            provider_id: "google".to_string(),
            name: "Gemini 2.5 Flash Preview".to_string(),
            family: Some("gemini-2.5".to_string()),
            context_window: 1_000_000,
            max_output_tokens: Some(65_536),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: None,
            release_date: Some("2025-05".to_string()),
        },
        ModelDefinition {
            id: "gemini-2.5-pro-preview-05-06".to_string(),
            provider_id: "google".to_string(),
            name: "Gemini 2.5 Pro Preview".to_string(),
            family: Some("gemini-2.5".to_string()),
            context_window: 1_000_000,
            max_output_tokens: Some(65_536),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: None,
            release_date: Some("2025-05".to_string()),
        },
        ModelDefinition {
            id: "gemini-1.5-pro".to_string(),
            provider_id: "google".to_string(),
            name: "Gemini 1.5 Pro".to_string(),
            family: Some("gemini-1.5".to_string()),
            context_window: 1_000_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: None,
            release_date: Some("2024-02".to_string()),
        },
        ModelDefinition {
            id: "gemini-1.5-flash".to_string(),
            provider_id: "google".to_string(),
            name: "Gemini 1.5 Flash".to_string(),
            family: Some("gemini-1.5".to_string()),
            context_window: 1_000_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: None,
            release_date: Some("2024-05".to_string()),
        },
        // ============================================
        // Groq (Fast Inference)
        // ============================================
        ModelDefinition {
            id: "llama-3.3-70b-versatile".to_string(),
            provider_id: "groq".to_string(),
            name: "Llama 3.3 70B Versatile".to_string(),
            family: Some("llama-3".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(32_768),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: Some("2024-12".to_string()),
            release_date: Some("2024-12".to_string()),
        },
        ModelDefinition {
            id: "llama-3.1-8b-instant".to_string(),
            provider_id: "groq".to_string(),
            name: "Llama 3.1 8B Instant".to_string(),
            family: Some("llama-3".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: false,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: Some("2024-07".to_string()),
            release_date: Some("2024-07".to_string()),
        },
        ModelDefinition {
            id: "mixtral-8x7b-32768".to_string(),
            provider_id: "groq".to_string(),
            name: "Mixtral 8x7B".to_string(),
            family: Some("mixtral".to_string()),
            context_window: 32_768,
            max_output_tokens: Some(4_096),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: false,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: None,
            release_date: Some("2024-01".to_string()),
        },
        // ============================================
        // Mistral AI
        // ============================================
        ModelDefinition {
            id: "mistral-large-latest".to_string(),
            provider_id: "mistral".to_string(),
            name: "Mistral Large".to_string(),
            family: Some("mistral-large".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities::full_featured(),
            knowledge_cutoff: None,
            release_date: Some("2024-02".to_string()),
        },
        ModelDefinition {
            id: "codestral-latest".to_string(),
            provider_id: "mistral".to_string(),
            name: "Codestral".to_string(),
            family: Some("codestral".to_string()),
            context_window: 32_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: None,
            release_date: Some("2024-05".to_string()),
        },
        ModelDefinition {
            id: "mistral-small-latest".to_string(),
            provider_id: "mistral".to_string(),
            name: "Mistral Small".to_string(),
            family: Some("mistral-small".to_string()),
            context_window: 32_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: false,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: None,
            release_date: Some("2024-09".to_string()),
        },
        // ============================================
        // DeepSeek
        // ============================================
        ModelDefinition {
            id: "deepseek-chat".to_string(),
            provider_id: "deepseek".to_string(),
            name: "DeepSeek V3".to_string(),
            family: Some("deepseek-v3".to_string()),
            context_window: 64_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: None,
            release_date: Some("2024-12".to_string()),
        },
        ModelDefinition {
            id: "deepseek-reasoner".to_string(),
            provider_id: "deepseek".to_string(),
            name: "DeepSeek R1".to_string(),
            family: Some("deepseek-r1".to_string()),
            context_window: 64_000,
            max_output_tokens: Some(8_192),
            capabilities: ModelCapabilities {
                tool_call: false, // R1 focuses on reasoning, limited tool support
                vision: false,
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: None,
            release_date: Some("2025-01".to_string()),
        },
        // ============================================
        // Together AI
        // ============================================
        ModelDefinition {
            id: "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".to_string(),
            provider_id: "together".to_string(),
            name: "Llama 3.1 405B Instruct Turbo".to_string(),
            family: Some("llama-3".to_string()),
            context_window: 128_000,
            max_output_tokens: Some(4_096),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: Some("2024-07".to_string()),
            release_date: Some("2024-07".to_string()),
        },
        ModelDefinition {
            id: "Qwen/Qwen2.5-Coder-32B-Instruct".to_string(),
            provider_id: "together".to_string(),
            name: "Qwen 2.5 Coder 32B".to_string(),
            family: Some("qwen-2.5".to_string()),
            context_window: 32_768,
            max_output_tokens: Some(4_096),
            capabilities: ModelCapabilities {
                tool_call: true,
                vision: false,
                reasoning: true,
                streaming: true,
                json_mode: true,
            },
            knowledge_cutoff: None,
            release_date: Some("2024-09".to_string()),
        },
    ]
}

/// Get all statically defined providers.
///
/// These are the LLM providers we support out of the box.
/// Each provider has different authentication methods, API formats,
/// and model capabilities.
pub fn static_providers() -> Vec<ProviderDefinition> {
    vec![
        ProviderDefinition {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            api_base_url: "https://api.anthropic.com/v1".to_string(),
            env_vars: vec!["ANTHROPIC_API_KEY".to_string()],
            auth_type: AuthType::Both,
            is_openai_compatible: false,
            doc_url: Some("https://docs.anthropic.com/".to_string()),
        },
        ProviderDefinition {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            api_base_url: "https://api.openai.com/v1".to_string(),
            env_vars: vec!["OPENAI_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://platform.openai.com/docs/".to_string()),
        },
        ProviderDefinition {
            id: "google".to_string(),
            name: "Google".to_string(),
            api_base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            env_vars: vec!["GOOGLE_API_KEY".to_string(), "GEMINI_API_KEY".to_string()],
            auth_type: AuthType::Both,
            is_openai_compatible: false,
            doc_url: Some("https://ai.google.dev/docs".to_string()),
        },
        // Alias for API key provider compatibility
        ProviderDefinition {
            id: "google_ai".to_string(),
            name: "Google AI".to_string(),
            api_base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            env_vars: vec!["GOOGLE_API_KEY".to_string(), "GEMINI_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: false,
            doc_url: Some("https://ai.google.dev/docs".to_string()),
        },
        ProviderDefinition {
            id: "groq".to_string(),
            name: "Groq".to_string(),
            api_base_url: "https://api.groq.com/openai/v1".to_string(),
            env_vars: vec!["GROQ_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://console.groq.com/docs/".to_string()),
        },
        ProviderDefinition {
            id: "mistral".to_string(),
            name: "Mistral AI".to_string(),
            api_base_url: "https://api.mistral.ai/v1".to_string(),
            env_vars: vec!["MISTRAL_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.mistral.ai/".to_string()),
        },
        ProviderDefinition {
            id: "deepseek".to_string(),
            name: "DeepSeek".to_string(),
            api_base_url: "https://api.deepseek.com/v1".to_string(),
            env_vars: vec!["DEEPSEEK_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://platform.deepseek.com/docs".to_string()),
        },
        ProviderDefinition {
            id: "together".to_string(),
            name: "Together AI".to_string(),
            api_base_url: "https://api.together.xyz/v1".to_string(),
            env_vars: vec!["TOGETHER_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.together.ai/".to_string()),
        },
        // Alias for API key provider compatibility
        ProviderDefinition {
            id: "together_ai".to_string(),
            name: "Together AI".to_string(),
            api_base_url: "https://api.together.xyz/v1".to_string(),
            env_vars: vec!["TOGETHER_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.together.ai/".to_string()),
        },
        ProviderDefinition {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            api_base_url: "https://openrouter.ai/api/v1".to_string(),
            env_vars: vec!["OPENROUTER_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://openrouter.ai/docs".to_string()),
        },
        ProviderDefinition {
            id: "ollama".to_string(),
            name: "Ollama (Local)".to_string(),
            api_base_url: "http://localhost:11434/v1".to_string(),
            env_vars: vec![],            // No API key needed for local
            auth_type: AuthType::ApiKey, // Technically no auth, but compatible
            is_openai_compatible: true,
            doc_url: Some("https://ollama.ai/".to_string()),
        },
        // ============================================
        // Additional API Key Providers
        // ============================================
        ProviderDefinition {
            id: "azure".to_string(),
            name: "Azure OpenAI".to_string(),
            api_base_url: "https://{resource}.openai.azure.com/openai/deployments/{deployment}"
                .to_string(),
            env_vars: vec!["AZURE_OPENAI_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some(
                "https://learn.microsoft.com/en-us/azure/ai-services/openai/".to_string(),
            ),
        },
        ProviderDefinition {
            id: "xai".to_string(),
            name: "xAI (Grok)".to_string(),
            api_base_url: "https://api.x.ai/v1".to_string(),
            env_vars: vec!["XAI_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.x.ai/".to_string()),
        },
        ProviderDefinition {
            id: "fireworks".to_string(),
            name: "Fireworks AI".to_string(),
            api_base_url: "https://api.fireworks.ai/inference/v1".to_string(),
            env_vars: vec!["FIREWORKS_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.fireworks.ai/".to_string()),
        },
        ProviderDefinition {
            id: "cerebras".to_string(),
            name: "Cerebras".to_string(),
            api_base_url: "https://api.cerebras.ai/v1".to_string(),
            env_vars: vec!["CEREBRAS_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://inference-docs.cerebras.ai/".to_string()),
        },
        ProviderDefinition {
            id: "cohere".to_string(),
            name: "Cohere".to_string(),
            api_base_url: "https://api.cohere.com/compatibility/v1".to_string(),
            env_vars: vec!["COHERE_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.cohere.com/".to_string()),
        },
        ProviderDefinition {
            id: "perplexity".to_string(),
            name: "Perplexity".to_string(),
            api_base_url: "https://api.perplexity.ai".to_string(),
            env_vars: vec!["PERPLEXITY_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.perplexity.ai/".to_string()),
        },
        ProviderDefinition {
            id: "deepinfra".to_string(),
            name: "DeepInfra".to_string(),
            api_base_url: "https://api.deepinfra.com/v1/openai".to_string(),
            env_vars: vec!["DEEPINFRA_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://deepinfra.com/docs".to_string()),
        },
        ProviderDefinition {
            id: "huggingface".to_string(),
            name: "Hugging Face".to_string(),
            api_base_url: "https://api-inference.huggingface.co/models".to_string(),
            env_vars: vec!["HUGGINGFACE_API_KEY".to_string(), "HF_TOKEN".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: false, // HuggingFace has its own API format
            doc_url: Some("https://huggingface.co/docs/api-inference/".to_string()),
        },
        ProviderDefinition {
            id: "siliconflow".to_string(),
            name: "SiliconFlow".to_string(),
            api_base_url: "https://api.siliconflow.cn/v1".to_string(),
            env_vars: vec!["SILICONFLOW_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.siliconflow.cn/".to_string()),
        },
        ProviderDefinition {
            id: "nebius".to_string(),
            name: "Nebius".to_string(),
            api_base_url: "https://api.studio.nebius.ai/v1".to_string(),
            env_vars: vec!["NEBIUS_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://studio.nebius.ai/docs/".to_string()),
        },
        ProviderDefinition {
            id: "nvidia".to_string(),
            name: "NVIDIA NIM".to_string(),
            api_base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            env_vars: vec!["NVIDIA_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.nvidia.com/nim/".to_string()),
        },
        ProviderDefinition {
            id: "sambanova".to_string(),
            name: "SambaNova".to_string(),
            api_base_url: "https://api.sambanova.ai/v1".to_string(),
            env_vars: vec!["SAMBANOVA_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.sambanova.ai/".to_string()),
        },
        ProviderDefinition {
            id: "hyperbolic".to_string(),
            name: "Hyperbolic".to_string(),
            api_base_url: "https://api.hyperbolic.xyz/v1".to_string(),
            env_vars: vec!["HYPERBOLIC_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://docs.hyperbolic.xyz/".to_string()),
        },
        ProviderDefinition {
            id: "novita".to_string(),
            name: "Novita AI".to_string(),
            api_base_url: "https://api.novita.ai/v3/openai".to_string(),
            env_vars: vec!["NOVITA_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: Some("https://novita.ai/docs/".to_string()),
        },
        ProviderDefinition {
            id: "aihubmix".to_string(),
            name: "AIHubMix".to_string(),
            api_base_url: "https://aihubmix.com/v1".to_string(),
            env_vars: vec!["AIHUBMIX_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: None,
        },
        ProviderDefinition {
            id: "synthetic".to_string(),
            name: "Synthetic".to_string(),
            api_base_url: "https://api.synthetic.dev/v1".to_string(),
            env_vars: vec!["SYNTHETIC_API_KEY".to_string()],
            auth_type: AuthType::ApiKey,
            is_openai_compatible: true,
            doc_url: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_models_not_empty() {
        let models = static_models();
        assert!(!models.is_empty(), "Static models should not be empty");
        assert!(models.len() >= 10, "Should have at least 10 models");
    }

    #[test]
    fn test_static_providers_not_empty() {
        let providers = static_providers();
        assert!(
            !providers.is_empty(),
            "Static providers should not be empty"
        );
        assert!(providers.len() >= 5, "Should have at least 5 providers");
    }

    #[test]
    fn test_claude_models_have_correct_context() {
        let models = static_models();
        let claude_models: Vec<_> = models
            .iter()
            .filter(|m| m.provider_id == "anthropic")
            .collect();

        assert!(!claude_models.is_empty());
        for model in claude_models {
            assert_eq!(
                model.context_window, 200_000,
                "Claude model {} should have 200k context",
                model.id
            );
        }
    }

    #[test]
    fn test_gemini_models_have_large_context() {
        let models = static_models();
        let gemini_models: Vec<_> = models
            .iter()
            .filter(|m| m.provider_id == "google")
            .collect();

        assert!(!gemini_models.is_empty());
        for model in gemini_models {
            assert_eq!(
                model.context_window, 1_000_000,
                "Gemini model {} should have 1M context",
                model.id
            );
        }
    }

    #[test]
    fn test_all_models_have_valid_provider() {
        let models = static_models();
        let providers = static_providers();
        let provider_ids: Vec<_> = providers.iter().map(|p| &p.id).collect();

        for model in &models {
            assert!(
                provider_ids.contains(&&model.provider_id),
                "Model {} has unknown provider {}",
                model.id,
                model.provider_id
            );
        }
    }

    #[test]
    fn test_canonical_ids_are_unique() {
        let models = static_models();
        let mut canonical_ids: Vec<_> = models.iter().map(|m| m.canonical_id()).collect();
        let original_len = canonical_ids.len();
        canonical_ids.sort();
        canonical_ids.dedup();

        assert_eq!(
            canonical_ids.len(),
            original_len,
            "All canonical IDs should be unique"
        );
    }

    #[test]
    fn test_openai_compatible_providers() {
        let providers = static_providers();
        let openai_compatible: Vec<_> = providers
            .iter()
            .filter(|p| p.is_openai_compatible)
            .collect();

        // OpenAI, Groq, Mistral, DeepSeek, Together, OpenRouter, Ollama should be compatible
        assert!(
            openai_compatible.len() >= 6,
            "Should have at least 6 OpenAI-compatible providers"
        );
    }
}
