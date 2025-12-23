use std::path::PathBuf;

use ticca_core::agents::AgentType;
use ticca_core::tools::{AgentCallEvent, AgentStreamEvent, SystemExecRequest, TodoListEvent};

use super::{ImageAttachment, RightSidebarTab, TodoNodeOption};

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Msg {
    // Chat input + message list
    InputChanged(String),
    SendMessage,
    ChatScrolled(iced::widget::scrollable::Viewport),
    CopyMessage(usize),
    ToggleRawView(usize),
    RawViewEditorAction(usize, iced::widget::text_editor::Action),
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
    Reasoning(String),
    StreamStats {
        chars_in_window: usize,
        window_ms: u64,
    },
    /// Token usage from the API response
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    PollStreamStats,
    AnimationTick,
    StopStreaming,

    // Agent selection + UI layout
    SwitchAgent(AgentType),
    ToggleFlowPanel,
    SelectSidebarTab(RightSidebarTab),
    SelectTodoNode(TodoNodeOption),

    // Working directory
    SelectWorkingDirectory,
    WorkingDirectoryChanged(PathBuf),

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
