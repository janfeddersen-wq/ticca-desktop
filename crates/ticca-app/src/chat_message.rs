//! Chat message types for display

use iced::widget::markdown;
use ticca_core::agents::AgentType;
use ticca_core::session::MessageRole;
use uuid::Uuid;

/// Stable identifier for a chat message (survives index changes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from string (for session loading)
    pub fn from_string(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A sub-agent message rendered as a collapsible inside the parent message
#[derive(Debug, Clone)]
pub struct SubAgentMessage {
    /// Unique node ID for this sub-agent invocation
    pub node_id: usize,
    /// Type of the sub-agent
    pub agent_type: AgentType,
    /// The sub-agent's output content
    pub content: String,
    /// Reasoning/thinking content (collapsible)
    pub reasoning: Option<String>,
    /// Whether this sub-agent is still streaming
    pub is_streaming: bool,
    /// Whether this sub-agent's content is collapsed in the UI
    pub collapsed: bool,
    /// Track if last content added was a tool call (for formatting)
    pub last_was_tool_call: bool,
    /// Parsed markdown items for the content
    pub parsed_items: Vec<markdown::Item>,
}

impl SubAgentMessage {
    pub fn new_streaming(node_id: usize, agent_type: AgentType) -> Self {
        Self {
            node_id,
            agent_type,
            content: String::new(),
            reasoning: None,
            is_streaming: true,
            collapsed: false,
            last_was_tool_call: false,
            parsed_items: Vec::new(),
        }
    }

    /// Update parsed items when content changes
    pub fn update_parsed_items(&mut self) {
        self.parsed_items = markdown::parse(&self.content).collect();
    }
}

/// A content block within a message - either text or a sub-agent
#[derive(Debug, Clone)]
pub enum ContentBlock {
    /// Main agent text content
    Text {
        content: String,
        parsed_items: Vec<markdown::Item>,
    },
    /// Sub-agent invocation (rendered as collapsible)
    SubAgent(SubAgentMessage),
}

impl ContentBlock {
    pub fn new_text() -> Self {
        Self::Text {
            content: String::new(),
            parsed_items: Vec::new(),
        }
    }

    pub fn text(content: String) -> Self {
        let parsed_items = markdown::parse(&content).collect();
        Self::Text {
            content,
            parsed_items,
        }
    }

    /// Update parsed items for text blocks
    pub fn update_parsed_items(&mut self) {
        if let Self::Text {
            content,
            parsed_items,
        } = self
        {
            *parsed_items = markdown::parse(content).collect();
        }
    }
}

/// A chat message for display
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Stable identifier (survives index changes)
    pub id: MessageId,
    pub role: MessageRole,
    /// Full content as plain text (for copying, session save)
    pub content: String,
    pub is_streaming: bool,
    pub author_label: Option<String>,
    /// Reasoning/thinking content (collapsible)
    pub reasoning: Option<String>,
    /// Signature for reasoning content (required by Claude for verification)
    pub reasoning_signature: Option<String>,
    /// Content blocks in order (text and sub-agents interleaved)
    pub content_blocks: Vec<ContentBlock>,
    /// Track if last content added was a tool call (for formatting)
    pub last_was_tool_call: bool,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: MessageId::new(),
            role: MessageRole::User,
            content: content.clone(),
            is_streaming: false,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            content_blocks: vec![ContentBlock::text(content)],
            last_was_tool_call: false,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: MessageId::new(),
            role: MessageRole::Assistant,
            content: content.clone(),
            is_streaming: false,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            content_blocks: vec![ContentBlock::text(content)],
            last_was_tool_call: false,
        }
    }

    pub fn assistant_streaming() -> Self {
        Self {
            id: MessageId::new(),
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            content_blocks: vec![ContentBlock::new_text()],
            last_was_tool_call: false,
        }
    }

    #[allow(dead_code)]
    pub fn assistant_streaming_named(label: impl Into<String>) -> Self {
        Self {
            id: MessageId::new(),
            role: MessageRole::Assistant,
            content: String::new(),
            is_streaming: true,
            author_label: Some(label.into()),
            reasoning: None,
            reasoning_signature: None,
            content_blocks: vec![ContentBlock::new_text()],
            last_was_tool_call: false,
        }
    }

    /// System message for notifications (compression, etc.)
    pub fn system(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: MessageId::new(),
            role: MessageRole::System,
            content: content.clone(),
            is_streaming: false,
            author_label: None,
            reasoning: None,
            reasoning_signature: None,
            content_blocks: vec![ContentBlock::text(content)],
            last_was_tool_call: false,
        }
    }

    /// Create message with a specific ID (for session loading)
    #[allow(dead_code)]
    pub fn with_id(mut self, id: MessageId) -> Self {
        self.id = id;
        self
    }

    /// Update parsed items when content changes
    pub fn update_parsed_items(&mut self) {
        for block in &mut self.content_blocks {
            block.update_parsed_items();
        }
    }

    /// Get the current (last) text block, creating one if needed
    pub fn current_text_block_mut(&mut self) -> &mut String {
        if !matches!(self.content_blocks.last(), Some(ContentBlock::Text { .. })) {
            self.content_blocks.push(ContentBlock::new_text());
        }

        match self.content_blocks.last_mut() {
            Some(ContentBlock::Text { content, .. }) => content,
            _ => unreachable!(),
        }
    }

    /// Add a sub-agent block
    pub fn add_sub_agent(&mut self, sub_agent: SubAgentMessage) {
        self.content_blocks.push(ContentBlock::SubAgent(sub_agent));
    }

    /// Find a sub-agent by node_id
    pub fn sub_agent_mut(&mut self, node_id: usize) -> Option<&mut SubAgentMessage> {
        self.content_blocks.iter_mut().find_map(|block| {
            if let ContentBlock::SubAgent(sub) = block
                && sub.node_id == node_id
            {
                return Some(sub);
            }
            None
        })
    }
}
