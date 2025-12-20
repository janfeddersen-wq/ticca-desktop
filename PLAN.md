# Refactor Plan (Maintainability Roadmap)

This document is a suggested, incremental refactor plan for improving long-term maintainability while keeping behavior stable.

## Goals

- Make the codebase easier to navigate by tightening module boundaries and adopting feature-oriented structure.
- Reduce “god files” and ripple effects from common changes.
- Improve testability by separating pure state updates from side effects (IO, async, DB, OAuth).
- Keep responsibilities clear:
  - `crates/ticca-app/`: UI state, rendering, platform/UI adapters.
  - `crates/ticca-core/`: domain logic, persistence, tools, provider integration.
  - `crates/ticca-oauth/`: OAuth flows only (no app persistence rules).

## Current Snapshot (quick audit)

- `crates/ticca-app/src/app.rs` (~1590 LOC) mixes: navigation, chat state machine, streaming lifecycle, tool approvals, model selection, OAuth triggers, session persistence, and system execution UI wiring.
- `crates/ticca-app/src/llm_stream.rs` (~1335 LOC) contains provider-specific streaming loops + tool wiring, and it depends on UI `Message` types (tight coupling).
- `crates/ticca-app/src/messages.rs` is a “mega enum” that forces unrelated features to share a single message namespace.
- App crate still performs persistence chores directly:
  - `crates/ticca-app/src/session_manager.rs` uses `SessionDatabase` directly.
  - `crates/ticca-app/src/oauth_handler.rs` writes OAuth accounts/tokens directly via `ConfigDatabase`.

These are typical maintainability pain points: “large modules”, “feature coupling”, and “UI ↔ domain” bleed-through.

## Target Architecture (end state)

### 1) Feature modules in `ticca-app`

Organize the UI around feature modules that each own:

- `State` (data for the feature)
- `Msg` (feature-local messages)
- `update(state, msg) -> (effects…)` (pure-ish reducer)
- `view(state) -> Element<Msg>` (or view functions already in `views/`)

Suggested layout:

- `crates/ticca-app/src/features/chat/`
- `crates/ticca-app/src/features/settings/`
- `crates/ticca-app/src/features/sessions/`
- `crates/ticca-app/src/features/oauth/`
- `crates/ticca-app/src/features/system_exec/`

Keep `crates/ticca-app/src/views/` as the rendering layer, but make views consume feature state rather than reaching across the whole app.

### 2) Hierarchical messages + routing

Evolve toward a routed message type:

- `enum Message { Chat(chat::Msg), Settings(settings::Msg), … }`

This localizes change: adding a chat-only message doesn’t require touching settings/system exec code.

### 3) Effects layer for side effects

Keep state updates separate from IO by returning “effects” from feature updates (your `AppCommand` enum is already close to this).

End goal:

- Feature update emits `Effect`s (start stream, persist session, open file picker, clipboard copy, etc.)
- `TiccaApp` executes effects and maps their completions back to messages.

### 4) Streaming engine lives in `ticca-core`

Move the provider/tool streaming engine out of `ticca-app/src/llm_stream.rs` into `ticca-core`, so:

- `ticca-core` owns: provider selection, multi-turn loop, todo-guard, tool wiring, approval gate logic, system exec requests.
- `ticca-app` owns: mapping core events to UI state and rendering.

Core should not depend on `iced` or on app `Message` types.

## Work Phases (incremental, low-risk)

### Phase 0 — Baseline & safety rails

- Record a baseline:
  - Size hotspots (`app.rs`, `llm_stream.rs`, `messages.rs`).
  - Main state machines (chat streaming lifecycle, approvals queue, session save triggers).
- Add/strengthen “smoke tests” where cheap:
  - Keep existing tests (e.g., in `crates/ticca-app/src/llm_stream.rs` and `crates/ticca-core/src/tools/mod.rs`) green throughout.
- Decide “what stays UI-specific” early:
  - `ChatMessage` likely stays in `ticca-app` because it includes parsed markdown items.

**Exit criteria:** clear boundaries + baseline tests pass.

### [x] Phase 1 — Modularize `ticca-app` (no behavior change)

Goal: shrink `TiccaApp::update` into a router and move logic into feature modules.

- Create feature module skeletons (`crates/ticca-app/src/app/features/chat.rs`, `crates/ticca-app/src/app/features/settings.rs`, …) with state + update functions.
- Move code out of `crates/ticca-app/src/app.rs` incrementally:
  - `ChatState` and chat-specific update arms into `crates/ticca-app/src/app/features/chat.rs`.
  - `SettingsState` + settings update arms into `crates/ticca-app/src/app/features/settings.rs`.
  - System execution UI state can wrap existing `SystemExecutionsState` from `crates/ticca-app/src/system_executions.rs`.
- Keep `AppCommand` execution centralized initially, but let features *emit* commands.
- Keep compilation stable by moving code without changing public APIs first.

**Exit criteria:** `crates/ticca-app/src/app.rs` becomes mostly routing + effect execution (target: < ~300 LOC), and behavior matches baseline.

### [x] Phase 2 — Untangle persistence responsibilities

Goal: stop doing DB write/read logic directly in UI.

- Sessions:
  - Introduce a core-facing session service in `crates/ticca-core/src/session/` (e.g., `SessionRepo` already exists; build on it).
  - Move “save/load session” orchestration out of `crates/ticca-app/src/session_manager.rs`.
  - Keep markdown parsing in `ticca-app` (convert raw session messages to `ChatMessage` at the boundary).
- OAuth persistence:
  - Move provider-specific “store token/account” logic out of `crates/ticca-app/src/oauth_handler.rs` into `crates/ticca-core/src/config/`.
  - Keep `ticca-oauth` responsible for fetching the token response; `ticca-core` responsible for persisting it.
- Normalize provider identifiers:
  - Avoid stringly-typed `"claude" / "gemini" / "chatgpt"` in app code.
  - Prefer `ticca_core::llm::ProviderId` (or a single shared enum) end-to-end.

**Exit criteria:** `ticca-app` no longer calls `ConfigDatabase::upsert_*` or `SessionDatabase::*` directly.

### [x] Phase 3 — Move streaming/agent execution into `ticca-core`

Goal: eliminate UI coupling from the streaming engine.

- Define a core event model (example shape):
  - `StreamEvent::Text`, `::Reasoning`, `::ToolCall`, `::ApprovalRequest`, `::AgentCall`, `::TodoEvent`, `::SystemExecRequest`, `::Done`, `::Error`.
- Implement a provider-agnostic runner in `ticca-core`:
  - Provider selection remains in core (`ProviderRegistry`).
  - Shared multi-turn loop + todo-guard lives in one place; provider-specific differences isolated in small modules.
  - Tool wiring (`create_tools`, approval gate, todo store) stays in core.
- `ticca-app` adapts:
  - Spawns the core runner task.
  - Subscribes to core events and translates them into feature messages.

**Exit criteria:** most of `crates/ticca-app/src/llm_stream.rs` is deleted or becomes a thin adapter; core runner is testable without `iced`.

### [x] Phase 4 — Cleanup, consistency, and tests

- Add reducer-style unit tests in `ticca-app` for the chat streaming lifecycle:
  - “SendMessage → start stream → chunks → complete/error → session save trigger”
  - Approval queue behavior (“request → show prompt → user decision → forward to runner”)
- Add focused tests in `ticca-core`:
  - todo-guard loop (max passes behavior)
  - provider selection and “no credentials” error paths
  - tool approval policy edge cases
- Remove duplication:
  - consolidate repeated “reset streaming state” logic into one helper
  - unify provider enums/types across crates

**Exit criteria:** smaller modules, fewer cross-module imports, tests cover the core state machines.

### [x] Phase 5 — Finish persistence boundary (config reads)

Goal: avoid opening SQLite directly in the UI layer for configuration reads, and make config loading a single core-owned API.

- Add a `ConfigService` API for loading `TypedSettings` + pinned models.
- Update `crates/ticca-app/src/app_config.rs` to call the service (keep `AppTheme::parse` in app).

**Exit criteria:** `crates/ticca-app/src/app_config.rs` no longer calls `ConfigDatabase::open`.

### [x] Phase 6 — Provider identity normalization

Goal: stop scattering `"claude" / "gemini" / "chatgpt"` string literals across the codebase and use the canonical constants.

- Replace app/core string literals with `ticca_core::config::models::providers::{CLAUDE, GEMINI, CHATGPT}`.
- Keep `ProviderId` ↔ provider-key conversions centralized.

**Exit criteria:** `rg '"claude"|"gemini"|"chatgpt"' crates/ticca-app/src` yields no provider-key literals in app code (except user-visible strings).

### [x] Phase 7 — Hierarchical messages + routing

Goal: split the `messages.rs` “mega enum” into feature-local message enums and route them through a small top-level `Message`.

- Introduce routed messages: `Message::Chat(chat::Msg)` and `Message::Settings(settings::Msg)`.
- Move message variants into `crates/ticca-app/src/messages/chat.rs` and `crates/ticca-app/src/messages/settings.rs`.
- Update app routing, views, subscriptions, and the core runner adapter to emit routed messages.

**Exit criteria:** `cargo check -p ticca-app` and `cargo test` pass, and message changes are localized to the owning feature.

### [x] Phase 8 — Effects layer cleanup (centralized side effects)

Goal: make side effects explicit and centralized so feature updates can focus on state transitions.

- Introduce `crates/ticca-app/src/app/effects.rs` with an `Effect` enum (stream start, OAuth, model refresh, file pickers, clipboard, open URL, focus/scroll).
- Change feature reducers to return `Vec<Effect>` instead of mutating a shared command buffer.
- Keep effect execution in one place (`TiccaApp` → `effects::task`), making future IO separation easier.

**Exit criteria:** `cargo check -p ticca-app` and `cargo test` pass, and feature updates no longer take `&mut Vec<…>` for effects.

## Suggested Sequencing (small commits that stay buildable)

1. Add `features/*` skeletons + route `TiccaApp::update` to them (keep existing logic in place).
2. Move chat state/update first (largest surface area), keeping `Message` variants stable.
3. Split `messages.rs` into feature-local enums and introduce `Message::Chat(chat::Msg)` routing.
4. Extract persistence services in `ticca-core` and switch call sites.
5. Extract core runner and replace the old streaming path.
6. Delete dead code and tighten visibility (`pub(crate)`), then add tests.

## Early Decisions (avoid churn)

- Where `ChatMessage` lives: keep it in `ticca-app` if it remains tied to `iced::widget::markdown` parsing.
- Event API shape from core runner:
  - `tokio::mpsc` events vs `impl Stream<Item = StreamEvent>` vs callback trait.
- Provider identity:
  - choose one canonical enum (`ProviderId`) and use it everywhere (UI + core + oauth mapping).
