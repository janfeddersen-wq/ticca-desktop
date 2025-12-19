# Improvement and Abstraction Plan

## Current Understanding (Architecture and Flow)

- `crates/ticca-app` is the Iced GUI. `app.rs` holds the main state machine, routes messages, drives streaming UI, manages working directory, and kicks off OAuth + model loads.
- `crates/ticca-core` contains business logic: agent definitions, tool implementations, config/session storage (SQLite), and LLM provider wrappers built on rig.
- `crates/ticca-oauth` owns PKCE OAuth flows and callback server support for Claude/Gemini/ChatGPT.
- LLM flow (today): `ticca-app` -> `llm_stream` -> provider selection by model name -> rig agent with tools -> tool wrappers -> native tools -> results -> UI.
- Tooling exists in both `ToolRegistry` and rig wrappers but now shares a central `tools/spec.rs`.
- OAuth token handling is centralized in `ticca-core::llm::auth` with cooldown-aware account selection.

## Goals

- Centralize provider and tool metadata so we define behavior once and adapt it to UI, rig, and storage.
- Make agent usage patterns explicit (tools, prompts, model strategy, safety) and reusable across UI and non-UI contexts.
- Reduce duplication and special-casing in `llm_stream.rs` and `app.rs`.
- Create clean seams for testing and future providers/tools.

## Status Snapshot

**Completed**
- Provider registry + model service + provider-based model refresh routing.
- OAuth multi-account support with cooldowns, priority ordering, last-used tracking, enable/disable, and UI controls.
- Yolo mode approval gating for shell/edit/delete tools with UI modal and approval queue.
- Settings UI tabbed layout with Accounts/Models/Tools/Appearance/Sessions.
- Tool spec module as single source of truth for registry + rig tool definitions, plus argument compatibility (`path` vs `file_path`, updated `grep` args).
- AgentProfile abstraction wired into app (system prompt + tool list + max rounds).
- Repository traits for config/session storage and re-exports.

**In Progress / Partial**
- Agent model strategy is basic (pinned > default); not yet multi-rule or provider-aware.
- Prompt reuse is still embedded in agent structs; no shared prompt block builder.
- Typed settings and defaults are partial (only a few keys are typed).

**Not Started**
- UI state decomposition (`ChatState`/`SettingsState`) and `AppCommand` boundary.
- Comprehensive test coverage (provider routing, tool specs, agent profiles, end-to-end harness).

## Plan

### Phase 1: Consolidate Provider/Model Abstractions

Status: **Completed**

- [x] **Provider registry**
   - Create a `ProviderRegistry` in `ticca-core` that defines:
     - provider id (`Claude`, `Gemini`, `ChatGPT`)
     - capabilities (supports_tools, supports_images, supports_reasoning, supports_streaming)
     - model naming rules (prefixes, normalizer)
     - auth requirements (access token vs id_token)
     - multi-account strategy hooks (cooldown, quota, priority)
   - Replace `llm_stream` model-name routing with registry lookup.
   - Expose a single `resolve_provider(model_name)` API.

- [x] **Unified token access**
   - Move token reads into `ticca-core::llm::auth` (new module):
     - `get_token(provider)` -> `AuthToken { access, id_token, expires_at }`
     - `list_tokens(provider)` -> `Vec<AuthToken>` (multi-account)
     - `select_token(provider, policy)` -> `AuthToken` (cooldown-aware)
   - `ticca-app/src/llm_stream.rs` calls only this API.

- [x] **Model discovery service**
   - Add `ModelService` in `ticca-core` that merges:
     - provider fetch
     - fallback defaults
     - config overrides
   - UI now calls `ModelService::fetch_all()` / `fetch_for(provider)` instead of per-provider branches.

- [x] **Cooldown-aware account rotation**
   - Cooldown, last error, last used, active flag, and priority are persisted per account.
   - Policy: on 429 or provider-specific cooldown response, mark account cooling.
   - Token selection picks the next eligible account, falling back to waiting.

### Phase 2: Tool Definitions as a Single Source of Truth

Status: **Completed**

- [x] **Tool spec module**
   - Introduce `tools/spec.rs` with a `ToolSpec` that includes:
     - name, description
     - typed args struct + JSON schema
     - executor hook (sync/async)
   - Generate:
     - `ToolDefinition` for `ToolRegistry`
     - rig `Tool` wrappers automatically
   - Remove duplicate parameter definitions between `file_ops.rs` and `rig_tools.rs`.

- [x] **Consistent argument naming**
   - Align `file_path` vs `path`, `directory`, `search_string`, etc.
   - Provide a compatibility shim for old naming (optional field aliasing) so existing prompts still work.

- [~] **Unified tool context**
   - `working_directory` and `approval_mode` are present.
   - `policy` (limits/allowed paths) still pending.

### Phase 3: Agent Profiles and Usage Patterns

Status: **Partial**

- [~] **AgentProfile abstraction**
   - Create an `AgentProfile` struct in `ticca-core::agents`:
     - name, system prompt, tool set, model strategy, max rounds
     - explicit usage policy for tools (read-before-write, file size limits)
     - account selection policy (cooldown aware, failover order)
   - Build `Coding` and `Planning` profiles from shared building blocks.
   - UI selects a profile instead of direct agent logic.

- [ ] **Reusable prompt blocks**
   - Move tool descriptions and guidelines into a shared prompt builder:
     - `PromptBlocks::tool_docs(tool_specs)`
     - `PromptBlocks::agent_guidelines(profile)`
   - Avoid duplicating the same text between agents.

- [~] **Model strategy abstraction**
   - Basic pinned/default rules are implemented; multi-rule strategy is pending.

### Phase 4: UI State Decomposition

Status: **Not Started**

- [ ] **Split `TiccaApp` state**
   - Move chat-specific state to `ChatState` (messages, streaming, stats).
   - Move settings state to `SettingsState` (theme, provider auth, models).
   - Keep `TiccaApp` as a thin coordinator.

- [x] **Settings UI restructure**
   - Add a tabbed settings layout:
     - Accounts (per-provider accounts, add/remove, cooldown status)
     - Models (defaults, pinned per agent)
     - Tools & Safety (yolo mode, tool approvals, limits)
     - Appearance (theme, fonts, layout)
   - Use clear grouping and short descriptions for each section.

- [ ] **Command/side-effect boundary**
   - Introduce a small `AppCommand` enum:
     - e.g., `Command::StartOAuth`, `Command::FetchModels`, `Command::StreamChat`
   - UI update returns commands; an executor maps them to `Task<Message>`.
   - Simplifies testing and makes usage patterns explicit.

### Phase 5: Storage and Configuration Consistency

Status: **Partial**

- [~] **Typed settings**
   - Add typed accessors (e.g., `get_theme()`, `get_max_tool_rounds()`).
   - Centralize defaults in `ticca-core::config::defaults`.
   - Add `yolo_mode_enabled` and `account_rotation_policy`.

- [x] **Repository pattern**
   - Implement `SessionRepo` and `ConfigRepo` traits.
   - Enable mock-backed tests and future migration to alternative storage.

- [x] **Approval gating (Yolo Mode)**
   - Add a global `YoloMode` toggle in settings:
     - When OFF, tools `edit_file`, `delete_file`, `shell` require approval.
   - UI flow: tool call -> modal prompt -> approve/deny -> continue/abort.
   - Persist decision per tool invocation, not global.

### Phase 6: Validation and Tests

Status: **Not Started**

- [ ] **Provider routing tests**
   - Ensure model name resolution picks the expected provider.
   - Ensure cooldown-aware account selection rotates correctly.
- [ ] **Tool spec tests**
   - Validate JSON schema generation and rig tool conversion.
- [ ] **Agent profile tests**
   - Ensure tool lists and prompts render correctly per profile.
- [ ] **End-to-end smoke**
   - A thin harness around `llm_stream` with mocked providers.

## Suggested Sequencing (Timeboxed)

- Week 1: Provider registry + auth unification + model service
- Week 2: Tool spec centralization + adapter generation
- Week 3: Agent profile refactor + prompt block builder
- Week 4: UI state decomposition + command boundary
- Week 5: Storage abstraction + typed settings + test coverage

## Expected Benefits

- Less duplication and fewer inconsistencies between core and UI layers.
- Easier to add providers/tools without touching multiple modules.
- More explicit and reusable “usage patterns” (agent profiles + prompt blocks).
- Improved testability across the LLM + tools stack.
