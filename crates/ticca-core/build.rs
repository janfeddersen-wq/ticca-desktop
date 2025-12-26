//! Build script for ticca-core
//!
//! Generates static model registry data from:
//! 1. data/context_windows.json - curated context window sizes
//! 2. data/api_cache.json - cached models.dev API data
//!
//! The generated code is included in src/registry/mod.rs via include!().

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    // Tell cargo to rerun if these files change
    println!("cargo:rerun-if-changed=data/context_windows.json");
    println!("cargo:rerun-if-changed=data/api_cache.json");
    println!("cargo:rerun-if-changed=data/context_overrides.json");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    // Load context windows (always available)
    let context_windows = load_context_windows(&manifest_dir);
    let default_context_window = context_windows.get("_default").copied().unwrap_or(100_000);

    // Load context overrides (for provider-specific limits like ChatGPT OAuth)
    let context_overrides = load_context_overrides(&manifest_dir);

    // Load API data (may be incomplete/sample)
    let api_data = load_api_data(&manifest_dir);

    // Generate the static data file
    generate_registry_code(
        &out_dir,
        &context_windows,
        &context_overrides,
        &api_data,
        default_context_window,
    );
}

/// Load context windows from the curated JSON file
fn load_context_windows(manifest_dir: &str) -> HashMap<String, u64> {
    let path = Path::new(manifest_dir).join("data/context_windows.json");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=Could not read context_windows.json: {}", e);
            return HashMap::new();
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            println!("cargo:warning=Could not parse context_windows.json: {}", e);
            return HashMap::new();
        }
    };

    let mut map = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            // Include _default but skip _comment
            if key == "_comment" {
                continue;
            }
            if let Some(n) = val.as_u64() {
                map.insert(key.clone(), n);
            }
        }
    }
    map
}

/// Load context overrides from context_overrides.json
///
/// Overrides can be:
/// - `model_id` - applies to all providers
/// - `provider:model_id` - applies only to specific provider
fn load_context_overrides(manifest_dir: &str) -> HashMap<String, u64> {
    let path = Path::new(manifest_dir).join("data/context_overrides.json");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=Could not read context_overrides.json: {}", e);
            return HashMap::new();
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "cargo:warning=Could not parse context_overrides.json: {}",
                e
            );
            return HashMap::new();
        }
    };

    let mut map = HashMap::new();
    if let Some(overrides) = value.get("overrides").and_then(|v| v.as_object()) {
        for (key, val) in overrides {
            // Skip comment keys
            if key.starts_with('_') {
                continue;
            }
            if let Some(n) = val.as_u64() {
                map.insert(key.clone(), n);
            }
        }
    }

    if !map.is_empty() {
        println!(
            "cargo:warning=Loaded {} context window overrides",
            map.len()
        );
    }

    map
}

/// Provider data from api_cache.json
#[derive(Debug)]
struct ProviderData {
    id: String,
    name: String,
    api_base_url: String,
    env_vars: Vec<String>,
    doc_url: Option<String>,
    models: Vec<ModelData>,
}

/// Model data from api_cache.json
#[derive(Debug)]
struct ModelData {
    id: String,
    name: String,
    family: Option<String>,
    tool_call: bool,
    vision: bool,
    reasoning: bool,
    knowledge_cutoff: Option<String>,
    release_date: Option<String>,
    /// Context window from limit.context (models.dev API)
    context_window: Option<u64>,
    /// Max output tokens from limit.output (models.dev API)
    max_output_tokens: Option<u64>,
}

/// Fallback API URLs for providers where models.dev data is missing the URL
fn fallback_api_url(provider_id: &str) -> Option<&'static str> {
    match provider_id {
        "openai" => Some("https://api.openai.com/v1"),
        "anthropic" => Some("https://api.anthropic.com/v1"),
        "google" => Some("https://generativelanguage.googleapis.com/v1beta"),
        "groq" => Some("https://api.groq.com/openai/v1"),
        "mistral" => Some("https://api.mistral.ai/v1"),
        "cohere" => Some("https://api.cohere.ai/v1"),
        "togetherai" => Some("https://api.together.xyz/v1"),
        "deepinfra" => Some("https://api.deepinfra.com/v1/openai"),
        "perplexity" => Some("https://api.perplexity.ai"),
        "xai" => Some("https://api.x.ai/v1"),
        "cerebras" => Some("https://api.cerebras.ai/v1"),
        "fireworks" => Some("https://api.fireworks.ai/inference/v1"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "openrouter" => Some("https://openrouter.ai/api/v1"),
        "sambanova" => Some("https://api.sambanova.ai/v1"),
        "hyperbolic" => Some("https://api.hyperbolic.xyz/v1"),
        _ => None,
    }
}

/// Load and parse the API cache JSON
fn load_api_data(manifest_dir: &str) -> Vec<ProviderData> {
    let path = Path::new(manifest_dir).join("data/api_cache.json");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=Could not read api_cache.json: {}", e);
            return Vec::new();
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            println!("cargo:warning=Could not parse api_cache.json: {}", e);
            return Vec::new();
        }
    };

    let Some(obj) = value.as_object() else {
        println!("cargo:warning=api_cache.json is not an object");
        return Vec::new();
    };

    let mut providers = Vec::new();

    for (provider_id, provider_value) in obj {
        // Skip metadata keys
        if provider_id.starts_with('_') {
            continue;
        }

        let Some(provider_obj) = provider_value.as_object() else {
            continue;
        };

        let id = provider_obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(provider_id)
            .to_string();

        let name = provider_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        let api_base_url = provider_obj
            .get("api")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| fallback_api_url(&id))
            .unwrap_or("")
            .to_string();

        let env_vars: Vec<String> = provider_obj
            .get("env")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let doc_url = provider_obj
            .get("doc")
            .and_then(|v| v.as_str())
            .map(String::from);

        let models = parse_models(provider_obj.get("models"));

        providers.push(ProviderData {
            id,
            name,
            api_base_url,
            env_vars,
            doc_url,
            models,
        });
    }

    providers
}

/// Parse models from a provider's "models" field
fn parse_models(models_value: Option<&serde_json::Value>) -> Vec<ModelData> {
    let Some(models_obj) = models_value.and_then(|v| v.as_object()) else {
        return Vec::new();
    };

    let mut models = Vec::new();

    for (model_id, model_value) in models_obj {
        let Some(model_obj) = model_value.as_object() else {
            continue;
        };

        let id = model_obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(model_id)
            .to_string();

        let name = model_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        let family = model_obj
            .get("family")
            .and_then(|v| v.as_str())
            .map(String::from);

        let tool_call = model_obj
            .get("tool_call")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Check modalities for vision support
        let vision = model_obj
            .get("modalities")
            .and_then(|v| v.get("input"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|v| v.as_str() == Some("image")))
            .unwrap_or(false);

        let reasoning = model_obj
            .get("reasoning")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let knowledge_cutoff = model_obj
            .get("knowledge")
            .and_then(|v| v.as_str())
            .map(String::from);

        let release_date = model_obj
            .get("release_date")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Read context window and max output from limit object (models.dev API)
        let limit_obj = model_obj.get("limit");
        let context_window = limit_obj
            .and_then(|l| l.get("context"))
            .and_then(|v| v.as_u64());
        let max_output_tokens = limit_obj
            .and_then(|l| l.get("output"))
            .and_then(|v| v.as_u64());

        models.push(ModelData {
            id,
            name,
            family,
            tool_call,
            vision,
            reasoning,
            knowledge_cutoff,
            release_date,
            context_window,
            max_output_tokens,
        });
    }

    models
}

/// Generate the Rust code for the registry
fn generate_registry_code(
    out_dir: &str,
    context_windows: &HashMap<String, u64>,
    context_overrides: &HashMap<String, u64>,
    api_data: &[ProviderData],
    default_context_window: u64,
) {
    let out_path = Path::new(out_dir).join("registry_generated.rs");
    let mut file = fs::File::create(&out_path).expect("Could not create registry_generated.rs");

    // Write header
    writeln!(file, "// Auto-generated by build.rs - DO NOT EDIT").unwrap();
    writeln!(
        file,
        "// Generated from data/context_windows.json and data/api_cache.json"
    )
    .unwrap();
    writeln!(file).unwrap();

    // Generate providers function
    writeln!(
        file,
        "/// Generated provider definitions from models.dev API"
    )
    .unwrap();
    writeln!(
        file,
        "pub fn generated_providers() -> Vec<ProviderDefinition> {{"
    )
    .unwrap();
    writeln!(file, "    vec![").unwrap();

    for provider in api_data {
        let env_vars = provider
            .env_vars
            .iter()
            .map(|s| format!("\"{}\".to_string()", s))
            .collect::<Vec<_>>()
            .join(", ");

        let doc_url = match &provider.doc_url {
            Some(url) => format!("Some(\"{}\".to_string())", url),
            None => "None".to_string(),
        };

        // Determine auth type and OpenAI compatibility based on provider ID
        let (auth_type, is_openai_compatible) = match provider.id.as_str() {
            "anthropic" => ("AuthType::Both", "false"),
            "openai" => ("AuthType::ApiKey", "true"),
            "google" => ("AuthType::Both", "false"),
            "groq" | "mistral" | "deepseek" | "together" | "openrouter" | "xai" | "fireworks"
            | "cerebras" | "perplexity" | "cohere" | "sambanova" | "hyperbolic" => {
                ("AuthType::ApiKey", "true")
            }
            _ => ("AuthType::ApiKey", "true"), // Default to API key, OpenAI-compatible
        };

        writeln!(file, "        ProviderDefinition {{").unwrap();
        writeln!(file, "            id: \"{}\".to_string(),", provider.id).unwrap();
        writeln!(
            file,
            "            name: \"{}\".to_string(),",
            escape_str(&provider.name)
        )
        .unwrap();
        writeln!(
            file,
            "            api_base_url: \"{}\".to_string(),",
            provider.api_base_url
        )
        .unwrap();
        writeln!(file, "            env_vars: vec![{}],", env_vars).unwrap();
        writeln!(file, "            auth_type: {},", auth_type).unwrap();
        writeln!(
            file,
            "            is_openai_compatible: {},",
            is_openai_compatible
        )
        .unwrap();
        writeln!(file, "            doc_url: {},", doc_url).unwrap();
        writeln!(file, "        }},").unwrap();
    }

    writeln!(file, "    ]").unwrap();
    writeln!(file, "}}").unwrap();
    writeln!(file).unwrap();

    // Generate models function
    writeln!(file, "/// Generated model definitions from models.dev API").unwrap();
    writeln!(file, "pub fn generated_models() -> Vec<ModelDefinition> {{").unwrap();
    writeln!(file, "    vec![").unwrap();

    for provider in api_data {
        for model in &provider.models {
            // Use context window from API (limit.context), fall back to manual lookup, then default
            let context_window = model
                .context_window
                .or_else(|| context_windows.get(&model.id).copied())
                .unwrap_or(default_context_window);

            // Use max output from API (limit.output), or estimate as 1/4 of context capped at 16k
            let max_output_tokens = model
                .max_output_tokens
                .unwrap_or_else(|| (context_window / 4).min(16_384));

            let family = match &model.family {
                Some(f) => format!("Some(\"{}\".to_string())", f),
                None => "None".to_string(),
            };

            let knowledge_cutoff = match &model.knowledge_cutoff {
                Some(k) => format!("Some(\"{}\".to_string())", k),
                None => "None".to_string(),
            };

            let release_date = match &model.release_date {
                Some(r) => format!("Some(\"{}\".to_string())", r),
                None => "None".to_string(),
            };

            writeln!(file, "        ModelDefinition {{").unwrap();
            writeln!(file, "            id: \"{}\".to_string(),", model.id).unwrap();
            writeln!(
                file,
                "            provider_id: \"{}\".to_string(),",
                provider.id
            )
            .unwrap();
            writeln!(
                file,
                "            name: \"{}\".to_string(),",
                escape_str(&model.name)
            )
            .unwrap();
            writeln!(file, "            family: {},", family).unwrap();
            writeln!(file, "            context_window: {},", context_window).unwrap();
            writeln!(
                file,
                "            max_output_tokens: Some({}),",
                max_output_tokens
            )
            .unwrap();
            writeln!(file, "            capabilities: ModelCapabilities {{").unwrap();
            writeln!(file, "                tool_call: {},", model.tool_call).unwrap();
            writeln!(file, "                vision: {},", model.vision).unwrap();
            writeln!(file, "                reasoning: {},", model.reasoning).unwrap();
            writeln!(file, "                streaming: true,").unwrap();
            writeln!(file, "                json_mode: true,").unwrap();
            writeln!(file, "            }},").unwrap();
            writeln!(file, "            knowledge_cutoff: {},", knowledge_cutoff).unwrap();
            writeln!(file, "            release_date: {},", release_date).unwrap();
            writeln!(file, "        }},").unwrap();
        }
    }

    writeln!(file, "    ]").unwrap();
    writeln!(file, "}}").unwrap();

    // Generate context window lookup map
    writeln!(file).unwrap();
    writeln!(file, "/// Context window lookup map (model_id -> tokens)").unwrap();
    writeln!(
        file,
        "pub fn generated_context_windows() -> std::collections::HashMap<&'static str, u64> {{"
    )
    .unwrap();
    writeln!(file, "    let mut map = std::collections::HashMap::new();").unwrap();

    for (model_id, context_window) in context_windows {
        if model_id.starts_with('_') {
            continue; // Skip _default, _comment
        }
        writeln!(
            file,
            "    map.insert(\"{}\", {});",
            model_id, context_window
        )
        .unwrap();
    }

    writeln!(file, "    map").unwrap();
    writeln!(file, "}}").unwrap();

    // Generate context override lookup map
    writeln!(file).unwrap();
    writeln!(
        file,
        "/// Context window overrides (provider:model_id or model_id -> tokens)"
    )
    .unwrap();
    writeln!(
        file,
        "/// These take precedence over API data for specific provider configurations"
    )
    .unwrap();
    writeln!(
        file,
        "pub fn generated_context_overrides() -> std::collections::HashMap<&'static str, u64> {{"
    )
    .unwrap();
    writeln!(file, "    let mut map = std::collections::HashMap::new();").unwrap();

    for (key, context_window) in context_overrides {
        writeln!(file, "    map.insert(\"{}\", {});", key, context_window).unwrap();
    }

    writeln!(file, "    map").unwrap();
    writeln!(file, "}}").unwrap();

    // Generate default context window constant
    writeln!(file).unwrap();
    writeln!(file, "/// Default context window for unknown models").unwrap();
    writeln!(
        file,
        "pub const GENERATED_DEFAULT_CONTEXT_WINDOW: u64 = {};",
        default_context_window
    )
    .unwrap();

    println!(
        "cargo:warning=Generated registry_generated.rs with {} providers and {} models",
        api_data.len(),
        api_data.iter().map(|p| p.models.len()).sum::<usize>()
    );
}

/// Escape special characters in strings for Rust string literals
fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
