//! Tool Approval Modal
//!
//! Modal dialog for approving or denying tool execution requests.

use gpui::{
    div, px, InteractiveElement, IntoElement, ParentElement, Styled,
};
use gpui_component::{h_flex, v_flex, ActiveTheme, button::{Button, ButtonVariants}};

use crate::app::{PendingApproval, TiccaApp};
use ticca_core::tools::ToolApprovalDecision as CoreToolApprovalDecision;

/// Render the tool approval modal overlay
pub fn render_approval_modal(
    approval: &PendingApproval,
    cx: &mut gpui::Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();
    let id = approval.id;
    let name = approval.name.clone();
    
    // Truncate args for display
    let args_preview = if approval.args.len() > 500 {
        format!("{}...", &approval.args[..500])
    } else {
        approval.args.clone()
    };

    // Semi-transparent overlay
    div()
        .id("approval-modal-overlay")
        .absolute()
        .inset_0()
        .bg(gpui::hsla(0.0, 0.0, 0.0, 0.6))
        .flex()
        .items_center()
        .justify_center()
        .child(
            // Modal card
            v_flex()
                .w(px(500.))
                .max_h(px(400.))
                .p_4()
                .gap_3()
                .rounded_lg()
                .bg(theme.background)
                .border_1()
                .border_color(theme.border)
                .shadow_lg()
                // Title
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.foreground)
                        .child("🔒 Tool Approval Required"),
                )
                // Tool name
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child("Tool:"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .bg(theme.muted)
                                .text_sm()
                                .font_family("monospace")
                                .text_color(theme.accent_foreground)
                                .child(name.clone()),
                        ),
                )
                // Arguments preview
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child("Arguments:"),
                        )
                        .child(
                            div()
                                .p_2()
                                .rounded_md()
                                .bg(theme.muted)
                                .max_h(px(150.))
                                .overflow_y_hidden()
                                .text_xs()
                                .font_family("monospace")
                                .text_color(theme.muted_foreground)
                                .child(args_preview),
                        ),
                )
                // Buttons
                .child(
                    h_flex()
                        .gap_2()
                        .mt_2()
                        .justify_end()
                        .child(
                            Button::new("deny-btn")
                                .label("Deny")
                                .danger()
                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                    // Send denial
                                    if let Some(tx) = &this.chat.approval_tx {
                                        let _ = tx.send(CoreToolApprovalDecision {
                                            id,
                                            approved: false,
                                        });
                                    }
                                    this.pending_approval = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("approve-btn")
                                .label("Approve")
                                .primary()
                                .on_click(cx.listener(move |this, _event, _window, cx| {
                                    // Send approval
                                    if let Some(tx) = &this.chat.approval_tx {
                                        let _ = tx.send(CoreToolApprovalDecision {
                                            id,
                                            approved: true,
                                        });
                                    }
                                    this.pending_approval = None;
                                    cx.notify();
                                })),
                        ),
                ),
        )
}
