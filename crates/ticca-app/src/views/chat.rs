//! Chat View - Main chat interface
//!
//! Renders the chat header, message list, and input area.

use gpui::{
    div, prelude::FluentBuilder as _, px, relative, AnyElement, Context, ElementId,
    InteractiveElement as _, IntoElement, ParentElement as _, ScrollWheelEvent,
    StatefulInteractiveElement as _, Styled as _, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    IconName,
    input::Input,
    scroll::{Scrollbar, ScrollbarAxis},
    tab::{Tab, TabBar},
    text::markdown,
    v_flex, ActiveTheme, Disableable as _, Sizable as _,
};

use ticca_core::agents::AgentType;
use ticca_core::session::MessageRole;

use crate::actions::*;
use crate::app::TiccaApp;
use crate::chat_message::ChatMessage;

// =============================================================================
// Chat View
// =============================================================================

/// Render the complete chat view
pub fn render_chat_view(
    app: &TiccaApp,
    window: &mut Window,
    cx: &mut Context<TiccaApp>,
) -> AnyElement {
    h_flex()
        .size_full()
        .overflow_hidden() // Prevent layout breakage from long content
        .bg(cx.theme().background)
        // Main chat area
        .child(
            v_flex()
                .flex_1()
                .h_full()
                .min_w_0() // Allow flex item to shrink below content size
                .overflow_hidden()
                .child(render_header(app, window, cx))
                .child(render_dir_bar(app, cx))
                .child(render_messages(app, cx))
                .child(render_input_area(app, window, cx)),
        )
        // Right sidebar (when visible)
        .when(app.chat.flow_panel_visible, |this| {
            this.child(super::sidebar::render_sidebar(app, window, cx))
        })
        .into_any_element()
}

// =============================================================================
// Header
// =============================================================================

/// Render the chat header with agent tabs and action buttons
fn render_header(
    app: &TiccaApp,
    _window: &mut Window,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();
    let current_agent = app.chat.current_agent;

    // Determine selected index
    let selected_index = match current_agent {
        AgentType::Coding => 0,
        AgentType::Planning => 1,
        _ => 0,
    };

    // Agent selection tabs
    let agent_tabs = TabBar::new("agent-tabs")
        .py_1()
        .pill()
        .selected_index(selected_index)
        .on_click(cx.listener(|this, idx: &usize, _, cx| {
            this.chat.current_agent = match idx {
                0 => AgentType::Coding,
                1 => AgentType::Planning,
                _ => AgentType::Coding,
            };
            cx.notify();
        }))
        .child(
            Tab::new()
                .label("Coding")
                .prefix(IconName::SquareTerminal),
        )
        .child(
            Tab::new()
                .label("Planning")
                .prefix(IconName::BookOpen),
        );

    // Token usage display (placeholder)
    let token_display = div()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child("0 tokens");

    // Action buttons
    let actions = h_flex()
        .gap_1()
        .child(
            Button::new("new-session")
                .icon(IconName::Plus)
                .ghost()
                .small()
                .tooltip("New Session (Ctrl+N)")
                .on_click(cx.listener(|this, _, window, cx| {
                    // Cancel any ongoing stream
                    if let Some(handle) = &this.chat.stream_handle {
                        handle.cancel();
                    }
                    this.chat.stream_handle = None;
                    // Clear messages and input
                    this.chat.messages.clear();
                    this.chat.pending_attachments.clear();
                    this.chat.is_streaming = false;
                    // Reset auto-scroll for new session
                    this.chat.auto_scroll_enabled = true;
                    this.chat.input_state.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                    });
                    cx.notify();
                })),
        )
        .child(
            Button::new("theme-toggle")
                .icon(IconName::Palette)
                .ghost()
                .small()
                .tooltip("Change Theme")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.theme = this.theme.next();
                    cx.dispatch_action(&SwitchTheme(this.theme.theme_name().to_string()));
                    cx.notify();
                })),
        )
        .child(
            Button::new("settings")
                .icon(IconName::Settings)
                .ghost()
                .small()
                .tooltip("Settings (Ctrl+,)")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.current_view = View::Settings;
                    // Refresh settings data
                    this.settings.refresh_accounts();
                    this.settings.refresh_mcp();
                    this.settings.refresh_sessions();
                    this.settings.refresh_provider_select(window, cx);
                    // Refresh model data
                    this.settings.load_default_model();
                    this.settings.load_cached_models();
                    this.settings.load_agent_pinned_models();
                    this.settings.refresh_model_select(window, cx);
                    cx.notify();
                })),
        )
        .child(
            Button::new("sidebar")
                .icon(IconName::PanelRight)
                .ghost()
                .small()
                .tooltip("Toggle Sidebar (Ctrl+B)")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.chat.flow_panel_visible = !this.chat.flow_panel_visible;
                    cx.notify();
                })),
        );

    // Header container - v_flex to stack main header + status row
    v_flex()
        .w_full()
        .border_b_1()
        .border_color(theme.border)
        // Top row with tabs and buttons
        .child(
            h_flex()
                .w_full()
                .h(px(52.))
                .px_4()
                .items_center()
                .justify_between()
                .bg(theme.title_bar)
                .child(agent_tabs)
                .child(token_display)
                .child(actions),
        )
        // Status row for streaming stats and context usage
        .child(
            h_flex()
                .w_full()
                .h(px(24.))
                .px_4()
                .items_center()
                .justify_between()
                .bg(theme.secondary)
                .border_t_1()
                .border_color(theme.border)
                // Left side: streaming stats
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        // Streaming speed
                        .when(app.chat.is_streaming, |this| {
                            this.child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child("⚡"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.foreground)
                                            .child(format!(
                                                "{:.0} chars/s",
                                                app.chat.streaming_chars_per_sec.unwrap_or(0.0)
                                            )),
                                    ),
                            )
                        })
                        // Total chars streamed
                        .when(
                            app.chat.is_streaming && app.chat.streaming_total_chars > 0,
                            |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(format!(
                                            "({} chars)",
                                            app.chat.streaming_total_chars
                                        )),
                                )
                            },
                        ),
                )
                // Right side: context window usage
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .when_some(app.chat.context_usage_percent, |this, percent| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Context:"),
                            )
                            .child(
                                // Progress bar container
                                div()
                                    .w(px(60.))
                                    .h(px(4.))
                                    .bg(theme.muted)
                                    .rounded_sm()
                                    .overflow_hidden()
                                    .child(
                                        // Progress bar fill
                                        div()
                                            .h_full()
                                            .w(relative(percent as f32 / 100.0))
                                            .bg(if percent > 80 {
                                                theme.danger
                                            } else if percent > 60 {
                                                theme.link
                                            } else {
                                                theme.primary
                                            })
                                            .rounded_sm(),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(if percent > 80 {
                                        theme.danger
                                    } else {
                                        theme.muted_foreground
                                    })
                                    .child(format!("{}%", percent)),
                            )
                        }),
                ),
        )
}

// =============================================================================
// Directory Bar
// =============================================================================

/// Render the working directory bar
fn render_dir_bar(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();
    let dir_display = app
        .chat
        .working_directory
        .to_string_lossy()
        .to_string();

    h_flex()
        .w_full()
        .h(px(36.))
        .px_4()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(theme.border)
        .bg(theme.secondary)
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("📁"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .overflow_hidden()
                        .text_ellipsis()
                        .max_w(px(600.))
                        .child(dir_display),
                ),
        )
        .child(
            Button::new("select-dir")
                .label("Select Directory")
                .ghost()
                .xsmall()
                .on_click(cx.listener(|_, _, _, cx| {
                    cx.dispatch_action(&SelectWorkingDirectory);
                })),
        )
}

// =============================================================================
// Message List
// =============================================================================

/// Render the scrollable message list
fn render_messages(app: &TiccaApp, cx: &mut Context<TiccaApp>) -> impl IntoElement {
    let theme = cx.theme();

    // Build message elements
    let messages: Vec<AnyElement> = app
        .chat
        .messages
        .iter()
        .enumerate()
        .map(|(idx, msg)| render_message_bubble(idx, msg, cx))
        .collect();

    // Empty state if no messages
    // NOTE: Return content directly - no wrapper div!
    // Nested overflow contexts break scrolling.
    if messages.is_empty() {
        div()
            .id("messages-container")
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background)
            .child(
                v_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .text_3xl()
                            .child("🐶"),
                    )
                    .child(
                        div()
                            .text_lg()
                            .text_color(theme.muted_foreground)
                            .child("Ready to help! What can I do for you?"),
                    ),
            )
            .into_any_element()
    } else {
        // Use custom scroll setup for programmatic control
        // Structure matches gpui-component's Scrollable implementation
        let scroll_handle = &app.chat.scroll_handle;
        
        // Outer container - matches the flex layout context
        div()
            .id("messages-container")
            .flex_1()
            .min_h_0() // Critical for flex scroll containers!
            .size_full()
            .bg(theme.background)
            .relative()
            .child(
                // Scroll area - matches Scrollable's inner structure exactly
                div()
                    .id("scroll-area")
                    .flex()
                    .size_full()
                    .track_scroll(scroll_handle)
                    .flex_col()
                    .overflow_y_scroll()
                    .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                        // Check if user scrolled up (away from bottom)
                        // In GPUI, scrolling down produces negative delta.y
                        // When delta.y > 0, user is scrolling up (towards top)
                        let delta_y = event.delta.pixel_delta(px(1.)).y;
                        let zero = px(0.);
                        
                        if delta_y > zero {
                            // User scrolled up - disable auto-scroll
                            this.chat.auto_scroll_enabled = false;
                        } else {
                            // User scrolled down - check if at bottom
                            let offset = this.chat.scroll_handle.offset();
                            let max_offset = this.chat.scroll_handle.max_offset();
                            
                            // At bottom when offset.y is close to -max_offset.height
                            // (within ~50px threshold)
                            // offset.y is negative when scrolled down, so we negate it
                            let scroll_pos = -offset.y;
                            let threshold = px(50.);
                            let at_bottom = scroll_pos + threshold >= max_offset.height;
                            if at_bottom {
                                this.chat.auto_scroll_enabled = true;
                            }
                        }
                        cx.notify();
                    }))
                    .child(
                        // Content wrapper with flex_1() - matches Scrollable
                        v_flex()
                            .flex_1()
                            .p_4()
                            .gap_4()
                            .children(messages)
                    ),
            )
            // Scrollbar overlay - positioned absolutely
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .child(
                        Scrollbar::new(scroll_handle)
                            .id("messages-scrollbar")
                            .axis(ScrollbarAxis::Vertical)
                    )
            )
            .into_any_element()
    }
}

/// Render a single message bubble
fn render_message_bubble(
    idx: usize,
    msg: &ChatMessage,
    cx: &Context<TiccaApp>,
) -> AnyElement {
    let theme = cx.theme();
    let is_user = msg.role == MessageRole::User;
    let is_system = msg.role == MessageRole::System;

    // Role label
    let role_label = if is_user {
        "You"
    } else if is_system {
        "System"
    } else {
        msg.author_label.as_deref().unwrap_or("Assistant")
    };

    // Bubble styling based on role
    let (bubble_bg, text_color) = if is_user {
        (theme.accent, theme.accent_foreground)
    } else if is_system {
        (theme.muted, theme.muted_foreground)
    } else {
        (theme.secondary, theme.secondary_foreground)
    };

    // Streaming indicator
    let streaming_indicator = if msg.is_streaming {
        Some(
            div()
                .text_xs()
                .text_color(theme.primary)
                .child("● Streaming..."),
        )
    } else {
        None
    };

    // Message content - use markdown for rendering
    let content_element = if msg.content.is_empty() && msg.is_streaming {
        div()
            .text_sm()
            .text_color(text_color)
            .child("Thinking...")
            .into_any_element()
    } else {
        markdown(&msg.content)
            .selectable(true)
            .into_any_element()
    };

    // Build the message container
    v_flex()
        .id(ElementId::NamedInteger("msg".into(), idx as u64))
        .mb_4()
        .when(is_user, |this| this.items_end())
        .child(
            // Role label row
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.muted_foreground)
                        .child(role_label.to_string()),
                )
                .children(streaming_indicator),
        )
        .child(
            // Message bubble
            div()
                .mt_1()
                .p_3()
                .rounded_lg()
                .bg(bubble_bg)
                .text_color(text_color) // Ensure text color is inherited by all children (including markdown)
                .overflow_hidden() // Clip overflow content (long code blocks, URLs)
                .min_w_0() // Allow shrinking below content size
                .when(is_user, |this| this.max_w(px(600.)))
                .when(!is_user, |this| this.w_full().max_w_full())
                .child(content_element),
        )
        .into_any_element()
}

// =============================================================================
// Input Area
// =============================================================================

/// Render the message input area
fn render_input_area(
    app: &TiccaApp,
    _window: &mut Window,
    cx: &mut Context<TiccaApp>,
) -> impl IntoElement {
    let theme = cx.theme();
    let is_streaming = app.chat.is_streaming;
    let has_input = !app.chat.input_state.read(cx).value().is_empty();

    // Attachment count badge
    let attachment_badge = if !app.chat.pending_attachments.is_empty() {
        Some(
            div()
                .absolute()
                .top_neg_1()
                .right_neg_1()
                .w(px(16.))
                .h(px(16.))
                .rounded_full()
                .bg(theme.primary)
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(theme.primary_foreground)
                .child(app.chat.pending_attachments.len().to_string()),
        )
    } else {
        None
    };

    // Input container
    h_flex()
        .w_full()
        .p_3()
        .gap_2()
        .items_end()
        .border_t_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(
            // Attach button
            div()
                .relative()
                .child(
                    Button::new("attach")
                        .icon(IconName::File)
                        .ghost()
                        .on_click(cx.listener(|_, _, _, cx| {
                            cx.dispatch_action(&AttachImage);
                        })),
                )
                .children(attachment_badge),
        )
        .child(
            // Text input using gpui-component's Input
            Input::new(&app.chat.input_state)
                .cleanable(true)
                .flex_1(),
        )
        .child(
            // Send/Stop button
            if is_streaming {
                Button::new("stop")
                    .icon(IconName::CircleX)
                    .danger()
                    .rounded_full()
                    .tooltip("Stop generation")
                    .on_click(|_, _, cx| {
                        cx.dispatch_action(&StopStreaming);
                    })
                    .into_any_element()
            } else {
                Button::new("send")
                    .icon(IconName::ArrowUp)
                    .primary()
                    .rounded_full()
                    .disabled(!has_input)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.handle_send_message(window, cx);
                    }))
                    .into_any_element()
            },
        )
}
