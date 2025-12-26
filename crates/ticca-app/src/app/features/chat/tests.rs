//! Tests for chat state functionality.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::app_config::AppConfig;
use crate::theme::AppTheme;

use super::state::ChatState;
use super::types::ToolApprovalPrompt;

fn test_config() -> AppConfig {
    AppConfig {
        theme: AppTheme::Dark,
        default_model: None,
        agent_pinned_models: HashMap::new(),
        max_tool_rounds: 10,
        yolo_mode_enabled: true,
        ui_mode: ticca_core::config::UiMode::Expert,
        external_tools_prompt_dismissed: false,
        update_check_skip_remaining: 0,
        update_check_dismissed_version: None,
    }
}

#[test]
fn reset_stream_runtime_state_clears_fields() {
    let mut state = ChatState::new(&test_config(), PathBuf::from("."));
    state.is_streaming = true;
    state.stream_start_time = Some(std::time::Instant::now());
    state.stream_chars_received = 123;
    state.current_tps = 7.0;
    state.tps_samples.push_back(1.0);
    state.last_bytes_time = Some(std::time::Instant::now());
    state.spinner_frame = 9;
    state.pending_approvals.push_back(ToolApprovalPrompt {
        id: 1,
        name: "x".to_string(),
        args: "{}".to_string(),
    });
    state.active_approval = state.pending_approvals.pop_front();

    state.reset_stream_runtime_state();

    assert!(!state.is_streaming);
    assert!(state.approval_tx.is_none());
    assert!(state.pending_approvals.is_empty());
    assert!(state.active_approval.is_none());
    assert!(state.stream_cancel.is_none());
    assert!(state.stream_start_time.is_none());
    assert_eq!(state.stream_chars_received, 0);
    assert_eq!(state.current_tps, 0.0);
    assert!(state.tps_samples.is_empty());
    assert!(state.last_bytes_time.is_none());
    assert_eq!(state.spinner_frame, 0);
}
