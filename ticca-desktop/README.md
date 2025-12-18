# Ticca Desktop

A modern, cross-platform desktop application for AI-assisted coding built with [Iced](https://github.com/iced-rs/iced) and Rust.

![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-blue)
![Tests](https://img.shields.io/badge/Tests-55_passing-green)

---

## ✨ Features

- **Coding Agent** - Writes, modifies, and executes code using best practices (DRY, YAGNI, SOLID)
- **Planning Agent** - Strategic task breakdown and execution roadmaps for complex projects
- 🔐 **OAuth Authentication** - Native support for Claude (Anthropic), Gemini (Google), and ChatGPT (OpenAI)
- 🎨 **Multiple Themes** - Dark, Light, and Zinc color schemes (Tailwind CSS-inspired)
- ⌨️ **Keyboard-First** - Full keyboard navigation and shortcuts
- 💾 **Session Persistence** - SQLite-backed conversation history
- 🔧 **Native Tools** - Blazing-fast file operations, ripgrep search, and shell command execution
- 🦀 **100% Rust** - No Electron, no JavaScript, just pure Rust performance

---

## 🚀 Quick Start

### Prerequisites

- **Rust 2024 Edition** (1.85+)
- **Linux** with Wayland/X11 (primary target, macOS/Windows should work but untested)

### Installation

```bash
# Clone the repository
git clone https://github.com/jan/ticca-desktop
cd ticca-desktop

# Build in release mode
cargo build --release

# Run the application
cargo run --release
```

The binary will be available at `target/release/ticca`.

### First Run

1. Launch Ticca Desktop
2. Click the ⚙️ settings button
3. Authenticate with your preferred LLM provider (Claude recommended)
4. Start using Ticca for your development tasks

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Send message |
| `Ctrl+N` | New session |
| `Ctrl+,` | Open settings |
| `Ctrl+T` | Toggle theme |
| `Escape` | Close panel |
| `Ctrl+1` | Switch to Coding agent |
| `Ctrl+2` | Switch to Planning agent |

---

## 🏗️ Architecture

Ticca Desktop is organized as a Cargo workspace with three main crates:

```
ticca-desktop/
├── crates/
│   ├── ticca-app/      # 🖥️  Iced GUI application
│   ├── ticca-core/     # 🧠 Business logic, agents, tools, LLM client
│   └── ticca-oauth/    # 🔐 OAuth implementations for LLM providers
├── Cargo.toml          # Workspace configuration
└── LICENSE             # MIT License
```

### Crate Overview

#### `ticca-app` - The GUI

The desktop application built with [Iced](https://github.com/iced-rs/iced), a cross-platform GUI framework.

**Key components:**
- `app.rs` - Main application state and message handling
- `theme/` - Dark, Light, and Zinc color schemes
- `views/` - Chat and configuration views
- `views/components/` - Reusable UI components (markdown renderer, syntax highlighting, emoji support)
- `keybindings.rs` - Keyboard shortcut definitions

#### `ticca-core` - The Brain

Core business logic including agents, tools, and LLM integration.

**Modules:**
- `agents/` - Planning and Coding agent definitions with system prompts
- `tools/` - Native Rust tools (file ops, grep, shell)
- `config/` - SQLite-backed configuration storage
- `session/` - Chat session persistence
- `llm/` - Claude API client with streaming support

#### `ticca-oauth` - Authentication

OAuth 2.0 implementations with PKCE support for secure authentication.

**Supported providers:**
- `claude.rs` - Anthropic Claude (public PKCE flow)
- `gemini.rs` - Google Gemini (requires OAuth credentials)
- `chatgpt.rs` - OpenAI ChatGPT (public PKCE flow)

---

## 🤖 Agents

### Coding Agent

Full read/write access to your codebase for code generation and modification.

**Capabilities:**
- Create, read, modify, and delete files
- Execute shell commands
- Search codebase with ripgrep
- Follow best practices (DRY, YAGNI, SOLID, Zen of Python)
- Keep files under 600 lines, refactoring when needed

**Available tools:** `list_files`, `read_file`, `edit_file`, `delete_file`, `grep`, `run_shell_command`, `agent_share_your_reasoning`

### Planning Agent

Strategic planner for complex tasks - read-only access for safe exploration.

**Capabilities:**
- Analyze project structure and requirements
- Create detailed execution roadmaps
- Identify dependencies and risks
- Suggest alternative approaches

**Available tools:** `list_files`, `read_file`, `grep`, `agent_share_your_reasoning`

---

## 🔧 Native Tools

All tools are implemented in pure Rust for maximum performance:

| Tool | Description |
|------|-------------|
| `list_files` | Directory listing with smart ignore patterns (.git, node_modules, target, etc.) |
| `read_file` | Read files with optional line range support |
| `edit_file` | Swiss-army file editor: create, overwrite, targeted replacements, or snippet deletion |
| `delete_file` | Remove files with diff generation |
| `grep` | Regex search using ripgrep libraries (grep-regex, grep-searcher, ignore) |
| `run_shell_command` | Execute shell commands with configurable timeout and working directory |
| `agent_share_your_reasoning` | Explicit reasoning/planning tool for agent transparency |

### `edit_file` Payload Types

```rust
// Create or overwrite a file
ContentPayload { file_path, content, overwrite: bool }

// Targeted text replacements (surgical edits)
ReplacementsPayload { file_path, replacements: [{ old_str, new_str }] }

// Remove specific text
DeleteSnippetPayload { file_path, delete_snippet }
```

---

## 💾 Data Storage

Configuration and sessions are stored in `~/.local/share/ticca-desktop/`:

| File | Contents |
|------|----------|
| `config.db` | Settings, model configs, OAuth tokens |
| `sessions.db` | Chat session history and messages |

Both databases use SQLite with automatic migrations.

---

## 🔐 OAuth Setup

### Claude (Anthropic) - Recommended

1. Open Settings (`Ctrl+,`)
2. Click "🔐 Claude OAuth"
3. Authorize in your browser
4. Tokens are stored securely in `config.db`

### Gemini (Google)

Requires a Google Cloud project with OAuth credentials configured:

1. Create OAuth 2.0 credentials in Google Cloud Console
2. Configure the credentials in Ticca settings
3. Click "🔐 Gemini OAuth"
4. Complete Google sign-in

### ChatGPT (OpenAI)

1. Open Settings (`Ctrl+,`)
2. Click "🔐 ChatGPT OAuth"
3. Authorize with your OpenAI account

---

## 🧪 Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p ticca-core
cargo test -p ticca-oauth

# Run with debug logging
RUST_LOG=debug cargo test
```

### Test Coverage

| Crate | Tests |
|-------|-------|
| ticca-core | 42 |
| ticca-oauth | 13 |
| **Total** | **55** |

### Building for Development

```bash
# Quick development build
cargo build

# Run with debug logging
RUST_LOG=debug cargo run

# Check all crates without building
cargo check --all
```

### Code Style Guidelines

Ticca follows these principles:

- **DRY** - Don't Repeat Yourself
- **YAGNI** - You Aren't Gonna Need It
- **SOLID** - Single responsibility, Open/closed, Liskov substitution, Interface segregation, Dependency inversion
- **Zen of Python** - Even in Rust! Simple is better than complex.
- **600-line limit** - Files should be split when they exceed this threshold

---

## 📦 Dependencies

### Core Dependencies

| Crate | Purpose |
|-------|---------|
| `iced` | Cross-platform GUI framework |
| `rig-core` | LLM framework with tool support |
| `tokio` | Async runtime |
| `rusqlite` | SQLite database |
| `reqwest` | HTTP client |
| `serde` / `serde_json` | Serialization |

### Search & Files

| Crate | Purpose |
|-------|---------|
| `grep-regex` | ripgrep regex engine |
| `grep-searcher` | ripgrep file searcher |
| `ignore` | .gitignore-aware file walking |

### UI Components

| Crate | Purpose |
|-------|---------|
| `pulldown-cmark` | Markdown parsing |
| `syntect` | Syntax highlighting |
| `twemoji-assets` | Emoji rendering |

---

## 🗺️ Roadmap

- [ ] Windows and macOS testing/support
- [ ] Plugin system for custom tools
- [ ] Multiple LLM provider support in a single session
- [ ] Code review agent
- [ ] Git integration agent
- [ ] Voice input support
- [ ] Project templates

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🙏 Credits

Built with love and these amazing projects:

- [Iced](https://github.com/iced-rs/iced) - Cross-platform GUI framework
- [rig](https://github.com/0xPlaygrounds/rig) - LLM framework
- [ripgrep](https://github.com/BurntSushi/ripgrep) - Blazing fast text search
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [syntect](https://github.com/trishume/syntect) - Syntax highlighting
- [Tailwind CSS](https://tailwindcss.com/) - Color scheme inspiration

---

<div align="center">

**Made with Rust by Jan**

</div>
