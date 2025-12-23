<div align="center">

# 🤖 Ticca Desktop

### AI-Powered Coding Assistant with Real Filesystem Access

A native desktop application that brings AI coding assistance directly to your machine. Built with [Iced](https://iced.rs/) and Rust for speed, security, and seamless integration with your development workflow.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.16.0-green.svg)](https://github.com/jan/ticca-desktop/releases)
[![CI](https://github.com/jan/ticca-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/jan/ticca-desktop/actions/workflows/ci.yml)

[Features](#-features) • [Installation](#-installation) • [Quick Start](#-quick-start) • [Documentation](#-documentation) • [Contributing](#-contributing)

</div>

---

## 🎯 What is Ticca?

Ticca is a **local-first** desktop application that connects you with powerful AI models (Claude, Gemini, ChatGPT) while giving them **real access to your filesystem**. Unlike browser-based tools, Ticca runs natively on your machine—the AI can read files, write code, execute shell commands, and search your codebase through native Rust tools.

### Why Choose Ticca?

| Feature | Benefit |
|---------|---------|
| 🔒 **Privacy-First** | Your code stays on your machine. Only prompts go to the LLM API. |
| ⚡ **Native Performance** | Built in Rust with the Iced GUI framework—fast startup, low memory. |
| 🛠️ **Real Filesystem Access** | AI agents can actually read, write, and modify your codebase. |
| 🔗 **Multi-Agent Orchestration** | Specialized agents can invoke each other for complex workflows. |
| 🎨 **Beautiful Themes** | 11 built-in themes including Catppuccin, Dracula, Nord, and more. |
| 🔐 **OAuth Authentication** | No API keys to manage—just click and authenticate. |
| 📦 **Cross-Platform** | Runs on Linux, macOS, and Windows. |

---

## ✨ Features

### 🤖 Multi-Provider AI Support

Authenticate with your preferred AI provider using secure OAuth PKCE—no API keys required:

| Provider | Model Examples | Status |
|----------|---------------|--------|
| **Claude** (Anthropic) | Claude Sonnet 4, Claude 3.5 | ✅ Fully Supported |
| **Gemini** (Google) | Gemini 2.0 Flash, Gemini Pro | ✅ Fully Supported |
| **ChatGPT** (OpenAI) | GPT-4o, GPT-4 Turbo | ✅ Fully Supported |

### 🧠 Intelligent Agent System

Ticca ships with specialized agents that understand their domain and can orchestrate complex tasks:

#### Coding Agent
The primary agent for code generation and modification. Capabilities include:
- Reading and writing files with intelligent diffing
- Executing shell commands in an integrated terminal
- Searching codebases with ripgrep-powered regex search
- Managing long-running processes
- Tracking tasks with a built-in to-do list

#### Planning Agent
Strategic task breakdown and project planning:
- Analyzes complex features and creates step-by-step roadmaps
- Identifies dependencies and potential blockers
- Can hand off implementation tasks to the Coding Agent

#### Skills Agent
Extends Ticca with Python-based capabilities:
- Runs specialized Python scripts for advanced tasks
- Manages Python virtual environments automatically
- Includes bundled skills for common operations

### 🛠️ Native Rust Tooling

Every tool is implemented in Rust for maximum performance and reliability:

#### File Operations
| Tool | Description |
|------|-------------|
| `list_files` | Recursively list files with intelligent filtering (ignores `node_modules`, `target`, etc.) |
| `read_file` | Read file contents with optional line-range selection |
| `write_file` | Create new files with automatic directory creation |
| `edit_file` | Targeted text replacements with diff generation |
| `delete_file` | Remove files with safety checks and diff output |

#### Search & Execution
| Tool | Description |
|------|-------------|
| `grep` | Blazing-fast regex search powered by ripgrep |
| `execute_shell` | Run commands in an integrated terminal with real-time output |
| `list_processes` | View all running terminal processes |
| `read_process_output` | Stream output from long-running processes |
| `kill_process` | Gracefully terminate processes |

#### Agent Coordination
| Tool | Description |
|------|-------------|
| `list_agents` | Discover available agents and their capabilities |
| `invoke_agent` | Delegate tasks to specialized agents with isolated context |
| `todo_read` / `todo_write` | Maintain agent-scoped task lists for complex workflows |

### 🔌 MCP (Model Context Protocol) Support

Extend Ticca's capabilities with external tools via the Model Context Protocol:

- Import MCP server configurations from Claude Desktop, Cursor, or custom JSON
- Support for both **stdio** and **HTTP** transports
- Automatic tool discovery and integration
- Manage servers through the Settings UI

### 📦 External Tool Management

Ticca can download and manage external tools on-demand:

| Tool | Purpose |
|------|---------|
| **UV** | Fast Python package installer (required for Skills) |
| **Pandoc** | Universal document converter |
| **Node.js** | JavaScript runtime for document generation |
| **LibreOffice** | Office suite for spreadsheet operations |

Tools are downloaded to `~/.local/share/ticca-desktop/tools/` and managed through the Settings UI.

### 🎨 Theme Gallery

Choose from 11 beautiful themes to match your editor:

| Dark Themes | Light Themes |
|-------------|--------------|
| Dark (default) | Light |
| Zinc | Catppuccin Latte |
| Dracula | Gruvbox Light |
| Nord | |
| Catppuccin Mocha | |
| Tokyo Night | |
| One Dark | |
| Gruvbox Dark | |

Toggle themes instantly with `Ctrl+T` or select from the Settings panel.

---

## 📥 Installation

### Prerequisites

- **Rust 2024 edition** (1.85 or later)
- Platform-specific dependencies (see below)

### Linux

Install system dependencies first:

```bash
# Ubuntu/Debian
sudo apt install libxkbcommon-dev libwayland-dev libgtk-3-dev pkg-config libssl-dev

# Fedora
sudo dnf install libxkbcommon-devel wayland-devel gtk3-devel openssl-devel

# Arch Linux
sudo pacman -S libxkbcommon wayland gtk3 openssl
```

### macOS

No additional dependencies required. Xcode Command Line Tools should be installed:

```bash
xcode-select --install
```

### Windows

No additional dependencies required. Ensure you have the Visual Studio C++ Build Tools installed.

### Building from Source

```bash
# Clone the repository
git clone https://github.com/jan/ticca-desktop
cd ticca-desktop

# Build in release mode (optimized)
cargo build --release

# Run the application
./target/release/ticca
```

### Pre-built Binaries

Download pre-built binaries from the [Releases](https://github.com/jan/ticca-desktop/releases) page:

| Platform | Download |
|----------|----------|
| Linux x86_64 | `ticca-linux-x86_64.tar.gz` |
| macOS x86_64 | `ticca-macos-x86_64.tar.gz` |
| macOS ARM64 | `ticca-macos-aarch64.tar.gz` |
| Windows x86_64 | `ticca-windows-x86_64.zip` |

---

## 🚀 Quick Start

### 1. Launch Ticca

```bash
./target/release/ticca
```

### 2. Authenticate with a Provider

Click on one of the provider buttons (Claude, Gemini, or ChatGPT) in the sidebar. A browser window will open for OAuth authentication.

### 3. Start Coding with AI

Type your request in the chat input and press `Ctrl+Enter` to send. The AI can now:

- Read and understand your codebase
- Write and modify files
- Run shell commands
- Search for patterns across your project

### Example Prompts

```
"Read the main.rs file and explain what this application does"

"Create a new Rust module for handling user authentication"

"Find all TODO comments in this project and list them"

"Run the test suite and fix any failing tests"

"Refactor the database module to use connection pooling"
```

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Send message |
| `Ctrl+N` | New chat session |
| `Ctrl+,` | Open settings |
| `Ctrl+T` | Cycle through themes |
| `Ctrl+1` | Switch to Coding Agent |
| `Ctrl+2` | Switch to Planning Agent |
| `Ctrl+V` | Paste (supports images) |
| `Escape` | Cancel current operation |

---

## 📁 Project Architecture

```
ticca-desktop/
├── crates/
│   ├── ticca-app/              # Desktop GUI application
│   │   ├── src/
│   │   │   ├── app.rs          # Main application state
│   │   │   ├── views/          # UI components (chat, settings, etc.)
│   │   │   ├── theme/          # Color schemes and styles
│   │   │   ├── widgets/        # Custom Iced widgets
│   │   │   └── messages.rs     # Application message types
│   │   └── assets/             # Fonts, icons, desktop files
│   │
│   ├── ticca-core/             # Business logic library
│   │   └── src/
│   │       ├── agents/         # Agent definitions and runner
│   │       ├── tools/          # Native Rust tool implementations
│   │       ├── llm/            # LLM provider integrations
│   │       ├── config/         # SQLite database and settings
│   │       ├── session/        # Chat history persistence
│   │       ├── skills/         # Python skill management
│   │       └── external_tools/ # On-demand tool downloads
│   │
│   └── ticca-oauth/            # OAuth PKCE implementation
│       └── src/
│           ├── claude.rs       # Anthropic OAuth flow
│           ├── gemini.rs       # Google OAuth flow
│           ├── chatgpt.rs      # OpenAI OAuth flow
│           └── callback_server.rs  # Local callback handler
│
├── vendor/                     # Patched dependencies
│   └── rig-core/               # LLM framework (patched for OAuth)
│
├── docs/                       # Additional documentation
└── skills.zip                  # Bundled Python skills
```

---

## 💾 Data Storage

Ticca stores data in platform-specific directories:

| Platform | Location |
|----------|----------|
| **Linux** | `~/.local/share/ticca-desktop/` |
| **macOS** | `~/Library/Application Support/ticca-desktop/` |
| **Windows** | `%APPDATA%\ticca-desktop\` |

### Directory Structure

```
ticca-desktop/
├── config.db           # Settings and OAuth tokens (SQLite)
├── sessions.db         # Chat history (SQLite)
├── skills/             # Extracted Python skills
├── tools/              # Downloaded external tools
├── bin/                # Extracted binaries (UV, etc.)
└── venvs/              # Python virtual environments
```

---

## 🔧 Configuration

### Settings UI

Access settings with `Ctrl+,` or click the gear icon. Available options:

- **Theme**: Select from 11 color schemes
- **Default Model**: Choose your preferred AI model
- **YOLO Mode**: Skip tool approval prompts (enabled by default)
- **Expert Mode**: Show advanced UI sections
- **Max Tool Rounds**: Limit agent iterations (default: 500)
- **Account Rotation**: How to handle multiple OAuth accounts

### MCP Server Configuration

Import MCP servers from:
- Claude Desktop (`~/.config/claude/claude_desktop_config.json`)
- Cursor (`.cursor/mcp.json`)
- Custom JSON configuration

Example MCP configuration:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/allowed/dir"]
    }
  }
}
```

---

## 🛠️ Development

### Running in Development Mode

```bash
# With debug logging
RUST_LOG=debug cargo run -p ticca-app

# With info-level logging
RUST_LOG=info cargo run -p ticca-app
```

### Running Tests

```bash
# All tests
cargo test

# Core library tests only
cargo test -p ticca-core

# With output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Run clippy lints
cargo clippy --all-targets --all-features -- -D warnings
```

### Building for Release

```bash
# Optimized release build with LTO
cargo build --release

# The binary will be at:
# ./target/release/ticca
```

---

## 🤝 Contributing

Contributions are welcome! Please read our guidelines before submitting:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feat/amazing-feature`)
3. **Commit** your changes using [Conventional Commits](https://www.conventionalcommits.org/)
   - `feat:` New features
   - `fix:` Bug fixes
   - `refactor:` Code refactoring
   - `docs:` Documentation updates
   - `ci:` CI/CD changes
4. **Push** to your branch (`git push origin feat/amazing-feature`)
5. **Open** a Pull Request

### Code Style

- Rust 2024 edition with standard `rustfmt` formatting
- Modules use `snake_case`, types use `PascalCase`
- Use `anyhow`/`thiserror` for error handling
- Use `tracing` for logging
- Keep UI logic in `ticca-app`, domain logic in `ticca-core`

See [AGENTS.md](AGENTS.md) for detailed coding guidelines.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [Iced](https://iced.rs/) - Cross-platform GUI library for Rust
- [Rig](https://github.com/0xPlaygrounds/rig) - LLM orchestration framework
- [ripgrep](https://github.com/BurntSushi/ripgrep) - Fast regex search
- [Catppuccin](https://github.com/catppuccin/catppuccin) - Soothing pastel theme
- [Dracula](https://draculatheme.com/) - Dark theme for developers

---

<div align="center">

**[Report Bug](https://github.com/jan/ticca-desktop/issues)** • **[Request Feature](https://github.com/jan/ticca-desktop/issues)** • **[Discussions](https://github.com/jan/ticca-desktop/discussions)**

Made with ❤️ in Rust

</div>
