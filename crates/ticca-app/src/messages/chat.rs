use std::path::PathBuf;

use ticca_core::agents::AgentType;
use ticca_core::tools::{AgentCallEvent, AgentStreamEvent, SystemExecRequest, TodoListEvent};

use crate::app::DiffModalState;
use crate::chat_message::MessageId;

use super::{ImageAttachment, RightSidebarTab, TodoNodeOption};

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Msg {
    // Chat input + message list
    InputChanged(String),
    SendMessage,
    ChatScrolled(iced::widget::scrollable::Viewport),
    CopyMessage(MessageId),
    ToggleRawView(MessageId),
    /// Toggle the collapsed state of a sub-agent within a message
    ToggleSubAgentCollapsed {
        msg_id: MessageId,
        node_id: usize,
    },
    RawViewEditorAction(MessageId, iced::widget::text_editor::Action),
    PaneResized(iced::widget::pane_grid::ResizeEvent),

    // Streaming + tool lifecycle
    StreamChunk(String),
    StreamComplete,
    StreamStopped,
    StreamError(String),
    ToolCall {
        name: String,
        args: String,
    },
    AgentCall(AgentCallEvent),
    SubagentStream(AgentStreamEvent),
    TodoEvent(TodoListEvent),
    ToolApprovalRequested {
        id: u64,
        name: String,
        args: String,
    },
    ToolApprovalDecision {
        id: u64,
        approved: bool,
    },
    Reasoning {
        text: String,
        signature: Option<String>,
    },
    StreamStats {
        chars_in_window: usize,
        window_ms: u64,
    },
    /// Token usage from the API response
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// Pre-request context estimate (calculated before sending to LLM)
    /// Note: context_window and usage_percent from rig are ignored; we use our own lookup
    #[allow(dead_code)]
    ContextEstimate {
        system_prompt_tokens: usize,
        tool_definitions_tokens: usize,
        messages_tokens: usize,
        total_tokens: usize,
        context_window: u64,
        usage_percent: u32,
    },
    /// Context compression was applied
    ContextCompressed {
        original_messages: usize,
        compressed_messages: usize,
        original_tokens: usize,
        compressed_tokens: usize,
        strategy: String,
    },
    /// Context usage warning (approaching limit) - deprecated, use ContextEstimate
    ContextUsageWarning {
        current_tokens: usize,
        threshold_tokens: u64,
        context_window: u64,
        usage_percent: u32,
    },
    PollStreamStats,
    AnimationTick,
    StopStreaming,

    // Agent lifecycle events (for Flow Graph)
    #[allow(dead_code)]
    AgentNodeCompleted {
        node_id: usize,
    },
    #[allow(dead_code)]
    AgentNodeFailed {
        node_id: usize,
    },
    #[allow(dead_code)]
    RunCompleted {
        run_id: usize,
    },
    FlowAnimationTick,

    // Agent selection + UI layout
    SwitchAgent(AgentType),
    ToggleFlowPanel,
    SelectSidebarTab(RightSidebarTab),
    SelectTodoNode(TodoNodeOption),

    // Working directory
    SelectWorkingDirectory,
    WorkingDirectoryChanged(PathBuf),

    // Diff modal
    /// Show the diff modal with the given state
    #[allow(dead_code)]
    ShowDiffModal(String),
    /// Close the diff modal
    CloseDiffModal,
    /// Cache a diff for later display
    #[allow(dead_code)]
    CacheDiff { id: String, state: DiffModalState },

    // Session
    NewSession,
    LoadSession(String),

    // Image attachments
    SelectImageFile,
    FileDropped(PathBuf),
    ImageLoaded(Result<ImageAttachment, String>),
    PasteImage,
    ImagePasted(Result<ImageAttachment, String>),
    RemoveAttachment(usize),

    // Markdown
    LinkClicked(iced::widget::markdown::Uri),

    // System executions
    SystemExecRequest(SystemExecRequest),
    SystemExecTerminalEvent(iced_term::Event),
    SystemExecNewTerminalNameChanged(String),
    SystemExecCreateUserTerminal,
    SystemExecCloseTerminal(String),
    SystemExecKillTerminal(String),
    SystemExecCopyTerminal(String),
}
