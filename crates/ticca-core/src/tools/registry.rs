//! Tool registry and base types

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            success: false,
            content: String::new(),
            error: Some(msg),
        }
    }
}

/// JSON Schema for tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameterSchema {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, ToolParameterSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ToolParameterSchema>>,
}

impl ToolParameterSchema {
    pub fn string(description: impl Into<String>) -> Self {
        Self {
            param_type: "string".to_string(),
            description: Some(description.into()),
            default: None,
            properties: None,
            required: None,
            items: None,
        }
    }
    
    pub fn integer(description: impl Into<String>) -> Self {
        Self {
            param_type: "integer".to_string(),
            description: Some(description.into()),
            default: None,
            properties: None,
            required: None,
            items: None,
        }
    }
    
    pub fn boolean(description: impl Into<String>) -> Self {
        Self {
            param_type: "boolean".to_string(),
            description: Some(description.into()),
            default: None,
            properties: None,
            required: None,
            items: None,
        }
    }
    
    pub fn with_default(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }
    
    pub fn object(properties: HashMap<String, ToolParameterSchema>, required: Vec<String>) -> Self {
        Self {
            param_type: "object".to_string(),
            description: None,
            default: None,
            properties: Some(properties),
            required: Some(required),
            items: None,
        }
    }
}

/// Tool definition for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameterSchema,
}

/// Type alias for async tool executor function
pub type ToolExecutor = Arc<
    dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<ToolResult>> + Send>> + Send + Sync
>;

/// A registered tool with its definition and executor
pub struct RegisteredTool {
    pub definition: ToolDefinition,
    pub executor: ToolExecutor,
}

/// Tool registry for managing available tools
pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    /// Register a new tool
    pub fn register(&mut self, definition: ToolDefinition, executor: ToolExecutor) {
        let name = definition.name.clone();
        self.tools.insert(name, RegisteredTool { definition, executor });
    }
    
    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.get(name)
    }
    
    /// Get all tool definitions (for LLM function calling)
    pub fn get_definitions(&self) -> Vec<&ToolDefinition> {
        self.tools.values().map(|t| &t.definition).collect()
    }
    
    /// Get tool names
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
    
    /// Execute a tool by name
    pub async fn execute(&self, name: &str, params: Value) -> Result<ToolResult> {
        match self.tools.get(name) {
            Some(tool) => (tool.executor)(params).await,
            None => Ok(ToolResult::error(format!("Unknown tool: {}", name))),
        }
    }
    
    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
    
    /// Get number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// Helper macro to create tool parameter schemas more easily
#[macro_export]
macro_rules! tool_params {
    ($($name:literal : $schema:expr),* $(,)?) => {{
        let mut props = std::collections::HashMap::new();
        $(props.insert($name.to_string(), $schema);)*
        props
    }};
}
