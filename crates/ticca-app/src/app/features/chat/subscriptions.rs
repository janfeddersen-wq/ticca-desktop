//! Subscription stream functions for system executions and terminal events.

use tokio::sync::mpsc;

use crate::messages::{Message, chat};
use ticca_core::tools::SystemExecRequest;

/// Subscription data for receiving system execution requests from LLM tools.
#[derive(Clone)]
pub(super) struct SystemExecRequestSubscriptionData {
    pub key: u64,
    pub rx: std::sync::Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<SystemExecRequest>>>,
}

impl std::hash::Hash for SystemExecRequestSubscriptionData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialEq for SystemExecRequestSubscriptionData {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for SystemExecRequestSubscriptionData {}

/// Stream that receives system execution requests and forwards them as messages.
pub(super) fn system_exec_request_stream(
    data: &SystemExecRequestSubscriptionData,
) -> iced::futures::stream::BoxStream<'static, Message> {
    let rx = data.rx.clone();
    Box::pin(iced::stream::channel(100, async move |mut output| {
        use iced::futures::SinkExt;

        loop {
            let req = {
                let mut rx = rx.lock().await;
                rx.recv().await
            };

            match req {
                Some(req) => {
                    let _ = output
                        .send(Message::Chat(chat::Msg::SystemExecRequest(req)))
                        .await;
                }
                None => break,
            }
        }
    }))
}

/// Subscription data for terminal backend events.
#[derive(Clone)]
pub(super) struct TerminalBackendSubscriptionData {
    pub terminal_id: u64,
    pub rx: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<iced_term::AlacrittyEvent>>>,
}

impl std::hash::Hash for TerminalBackendSubscriptionData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.terminal_id.hash(state);
    }
}

impl PartialEq for TerminalBackendSubscriptionData {
    fn eq(&self, other: &Self) -> bool {
        self.terminal_id == other.terminal_id
    }
}

impl Eq for TerminalBackendSubscriptionData {}

/// Stream that receives terminal backend events and forwards them as messages.
pub(super) fn terminal_backend_stream(
    data: &TerminalBackendSubscriptionData,
) -> iced::futures::stream::BoxStream<'static, Message> {
    let terminal_id = data.terminal_id;
    let rx = data.rx.clone();
    Box::pin(iced::stream::channel(100, async move |mut output| {
        use iced::futures::SinkExt;

        loop {
            let ev = {
                let mut rx = rx.lock().await;
                rx.recv().await
            };

            match ev {
                Some(ev) => {
                    let _ = output
                        .send(Message::Chat(chat::Msg::SystemExecTerminalEvent(
                            iced_term::Event::BackendCall(
                                terminal_id,
                                iced_term::backend::Command::ProcessAlacrittyEvent(ev),
                            ),
                        )))
                        .await;
                }
                None => break,
            }
        }
    }))
}
