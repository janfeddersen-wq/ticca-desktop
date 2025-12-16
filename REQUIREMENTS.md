# Product Requirements Document (PRD): Native AI Desktop Agent

## 1. Executive Summary
**Goal:** Build a high-performance, desktop-native AI chat application using **Rust** for the application shell and **Python (PydanticAI)** for the agentic logic.
**Target Performance:** 120FPS rendering on supported displays, instantaneous input response, and zero UI blocking during heavy AI processing.
**Key Architecture:** Rust (UI/OS integration) hosting an embedded Python interpreter (Logic) via PyO3.

---

## 2. Technology Stack Constraints
The developer must adhere to the following stack to ensure performance and interoperability:

*   **Core Language:** Rust (2021 edition or later).
*   **UI Engine:** **GPUI** (Recommended)
    *   *Constraint:* Must utilize GPU acceleration (Metal/Vulkan/OpenGL). DOM-based webviews (Electron/Tauri) are **not** permitted unless they can demonstrate stable 120FPS text rendering and layout.
*   **AI/Agent Framework:** **PydanticAI** (running in embedded Python).
*   **Bridge Layer:** **PyO3** + `pyo3-asyncio`.
*   **Local Database:** **SQLite** (via `sqlx` or `rusqlite`).
*   **Async Runtime:** **Tokio**.

---

## 3. Detailed Functional Requirements

### 3.1 The Rust-Python Bridge (PyO3)
The application acts as a host for a Python runtime.

*   **Embedded Runtime:** The Python interpreter must be bundled inside the final executable. The end-user **must not** be required to install Python manually. (Use tools like `PyOxidizer` or `indygreg/python-build-standalone`).
*   **Direct Execution:** Agent logic runs in-process (memory-to-memory), not over an HTTP server, to minimize latency.
*   **Type Sharing:**
    *   Use `serde` (Rust) and `pydantic` (Python) to strictly define the data passing between the two languages.
    *   Input to Python: `AgentRequest` struct (JSON compatible).
    *   Output from Python: `AgentResponse` or `StreamChunk`.

### 3.2 The PydanticAI Agent (Logic)
*   **ReAct Pattern:** Implement the logic using PydanticAI’s graph/agent system.
*   **Extensible Tooling:** The Python side must expose a clear directory/module structure where new tools (e.g., "Check Calendar", "Search Web") can be added easily.
*   **RunContext Streaming:**
    *   The Agent must utilize PydanticAI's streaming capabilities.
    *   As the LLM generates tokens or the Agent switches steps (Thinking -> Acting -> Observing), these state changes must be pushed to Rust immediately.
    *   **Requirement:** The bridge must implement a Rust `Callback` or `Channel` passed into Python so Python can `await send_update()` without blocking the GIL.

### 3.3 User Interface (GPU Native)
*   **Performance Budget:** The UI thread must never handle logic. All AI/DB operations must be offloaded to background `tokio` tasks. The target frame rate is 120FPS.
*   **Markdown Rendering:** The chat interface must support rich Markdown rendering (Headers, Code Blocks with syntax highlighting, Lists) drawn directly via the GPU renderer.
*   **Streaming UI:** The UI must handle partial text updates efficiently (appending tokens to the view buffer without re-layouting the entire history every millisecond).
*   **Look & Feel:**
    *   "Claude Desktop" aesthetic (Minimalist, typography-focused).
    *   Sticky prompt bar at the bottom. with image support
    *   Smooth, inertia-based scrolling.

### 3.4 Data Persistence & Configuration
*   **Storage Engine:** SQLite.
*   **Location:** The database must reside in the user's standard configuration directory (XDG Base Directory on Linux, `Library/Application Support` on macOS, `AppData` on Windows).
    *   *Implementation:* Use the `directories` crate to resolve this path dynamically.
*   **Schema:**
    *   `conversations` table (id, title, created_at).
    *   `messages` table (id, conversation_id, role, content, timestamp).
    *   `settings` table (key-value store for API keys, theme preferences, default model).
*   **Migrations:** The app must automatically run SQL migrations on startup if the DB schema changes.

---

## 4. Extensibility Requirements
To ensure the codebase remains maintainable and easy to extend:

1.  **Trait-Based Agents (Rust):** Create a Rust Trait `AgentController` that abstracts the Python calls. This allows us to potentially swap the Python backend for a pure Rust backend later without rewriting the UI.
2.  **Plugin Folder (Python):** The embedded Python environment should look at a specific user folder (e.g., `~/.config/myapp/plugins`) to load extra PydanticAI tools dynamically at runtime.

---

## 5. Implementation Best Practices (Developer Checklist)

The developer must verify the following before delivery:

*   **Concurrency:** Use `Arc<Mutex<>>` or `RwLock` sparingly. Prefer message passing (Channels) between the UI thread and the AI Worker thread.
*   **Safety:**
    *   **No `unwrap()` in production code.** All errors must be handled and propagated using the `thiserror` (lib) or `anyhow` (app) crates.
    *   The app must not crash if the OpenAI/Anthropic API returns a 500 error or if the network is down.
*   **Linting:** Code must pass `cargo clippy -- -D warnings` (strict mode).
*   **Blocking the Thread:**
    *   Python calls must be wrapped in `task::spawn_blocking` or handled via `pyo3_asyncio` to prevent the UI from freezing while Python calculates.

---

## 6. Deliverables
1.  Source code repository.
2.  CI/CD Workflow (GitHub Actions) that compiles the Rust binary + bundles the Python environment.
3.  A standalone executable ( `.app`, `.exe`, or binary) that runs without external dependencies.

---

### Technical Note for the Developer:
> *"Focus heavily on the **event loop**. The Rust Tokio runtime will drive the application. Initialize Python once on a separate thread. Use `pyo3_asyncio::tokio::get_runtime()` to ensure Python awaitables bridge correctly to Rust futures. Do not let the Python GIL block the GPU render loop."*
