use std::path::PathBuf;

use iced::widget::scrollable::AbsoluteOffset;
use iced::{Task, widget};

use tokio::sync::mpsc;

use crate::chat_message::ChatMessage;
use crate::image_handler;
use crate::llm_stream;
use crate::messages::{Message, OAuthProvider, chat, settings};
use crate::oauth_handler;
use crate::views::chat::CHAT_SCROLLABLE_ID;

use ticca_core::agents::AgentType;
use ticca_core::external_tools::ExternalToolId;
use ticca_core::llm::ProviderId;
use ticca_core::tools::{SystemExecRequest, SystemExecStore, TodoListState, ToolApprovalDecision};

#[derive(Debug)]
pub(in crate::app) enum Effect {
    RunStream {
        system_prompt: String,
        user_message: String,
        model_name: Option<String>,
        working_directory: PathBuf,
        max_tool_rounds: u32,
        history: Vec<ChatMessage>,
        initial_todo_state: Option<TodoListState>,
        image_data: Vec<(String, String)>,
        yolo_mode_enabled: bool,
        current_agent: AgentType,
        approval_rx: mpsc::UnboundedReceiver<ToolApprovalDecision>,
        cancel_rx: tokio::sync::oneshot::Receiver<()>,
        system_exec_store: std::sync::Arc<SystemExecStore>,
        system_exec_tx: mpsc::UnboundedSender<SystemExecRequest>,
    },
    ScrollToBottom,
    StartOAuth(OAuthProvider),
    RefreshModels,
    RefreshModelsForProvider(ProviderId),
    CopyToClipboard(String),
    PickWorkingDirectory,
    PickImageFile,
    LoadImage(PathBuf),
    PasteImage,
    OpenUrl(String),
    FocusTerminal(u64),
    RefreshExternalTools,
    InstallExternalTool(ExternalToolId),
    UninstallExternalTool(ExternalToolId),
    InstallAllMissingTools(Vec<ExternalToolId>),
}

pub(in crate::app) fn task(effect: Effect) -> Task<Message> {
    match effect {
        Effect::RunStream {
            system_prompt,
            user_message,
            model_name,
            working_directory,
            max_tool_rounds,
            history,
            initial_todo_state,
            image_data,
            yolo_mode_enabled,
            current_agent,
            approval_rx,
            cancel_rx,
            system_exec_store,
            system_exec_tx,
        } => Task::run(
            llm_stream::run_rig_agent_stream(
                system_prompt,
                user_message,
                model_name,
                working_directory,
                max_tool_rounds,
                history,
                initial_todo_state,
                image_data,
                yolo_mode_enabled,
                current_agent,
                approval_rx,
                cancel_rx,
                system_exec_store,
                system_exec_tx,
            ),
            |event| event,
        ),
        Effect::ScrollToBottom => widget::operation::scroll_to(
            widget::Id::new(CHAT_SCROLLABLE_ID),
            AbsoluteOffset {
                x: 0.0,
                y: f32::MAX,
            },
        ),
        Effect::StartOAuth(provider) => Task::perform(oauth_handler::start_oauth(provider), {
            move |result| Message::Settings(settings::Msg::OAuthComplete(provider, result))
        }),
        Effect::RefreshModels => Task::perform(
            async move { ticca_core::llm::ModelService::fetch_all().await },
            |result| Message::Settings(settings::Msg::ModelsLoaded(result)),
        ),
        Effect::RefreshModelsForProvider(provider) => Task::perform(
            async move { ticca_core::llm::ModelService::fetch_for(provider).await },
            |result| Message::Settings(settings::Msg::ModelsLoaded(result)),
        ),
        Effect::CopyToClipboard(content) => Task::perform(
            async move {
                use arboard::Clipboard;
                match Clipboard::new() {
                    Ok(mut clipboard) => {
                        if let Err(e) = clipboard.set_text(&content) {
                            tracing::error!("Failed to copy to clipboard: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to access clipboard: {}", e);
                    }
                }
            },
            |_| Message::Noop,
        ),
        Effect::FocusTerminal(terminal_id) => {
            widget::operation::focus(widget::Id::from(terminal_id.to_string()))
        }
        Effect::PickWorkingDirectory => Task::perform(
            async {
                let dialog = rfd::AsyncFileDialog::new()
                    .set_title("Select Working Directory")
                    .pick_folder()
                    .await;

                dialog.map(|handle| handle.path().to_path_buf())
            },
            |result| match result {
                Some(path) => Message::Chat(chat::Msg::WorkingDirectoryChanged(path)),
                None => Message::Noop,
            },
        ),
        Effect::PickImageFile => Task::perform(
            async {
                let dialog = rfd::AsyncFileDialog::new()
                    .set_title("Select Image")
                    .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
                    .pick_file()
                    .await;

                match dialog {
                    Some(handle) => {
                        let path = handle.path().to_path_buf();
                        image_handler::load_image_from_path(&path).await
                    }
                    None => Err("No file selected".to_string()),
                }
            },
            |result| Message::Chat(chat::Msg::ImageLoaded(result)),
        ),
        Effect::LoadImage(path) => Task::perform(
            async move { image_handler::load_image_from_path(&path).await },
            |result| Message::Chat(chat::Msg::ImageLoaded(result)),
        ),
        Effect::PasteImage => Task::perform(
            async { image_handler::paste_image_from_clipboard().await },
            |result| Message::Chat(chat::Msg::ImagePasted(result)),
        ),
        Effect::OpenUrl(url) => Task::perform(
            async move {
                if let Err(e) = open::that(&url) {
                    tracing::warn!("Failed to open URL {}: {}", url, e);
                }
            },
            |_| Message::Noop,
        ),
        Effect::RefreshExternalTools => Task::perform(
            async { crate::app::features::settings::load_external_tools_status().await },
            |statuses| Message::Settings(settings::Msg::ExternalToolsLoaded(statuses)),
        ),
        Effect::InstallExternalTool(tool_id) => Task::run(
            crate::app::features::settings::install_external_tool_stream(tool_id),
            |msg| msg,
        ),
        Effect::UninstallExternalTool(tool_id) => Task::perform(
            async move { crate::app::features::settings::uninstall_external_tool(tool_id).await },
            move |result| {
                Message::Settings(settings::Msg::ExternalToolUninstallComplete(
                    tool_id, result,
                ))
            },
        ),
        Effect::InstallAllMissingTools(tools) => Task::run(
            crate::app::features::settings::install_all_tools_stream(tools),
            |msg| msg,
        ),
    }
}
