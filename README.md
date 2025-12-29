<p align="center">
  <img src="crates/ticca-app/assets/icons/ticca-icon.png" alt="Ticca Logo" width="128" height="128">
</p>

<h1 align="center">Ticca Desktop</h1>

<p align="center">
  <strong>A native AI coding assistant with real filesystem access</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#ai-providers">AI Providers</a> •
  <a href="#agents">Agents</a> •
  <a href="#tools">Tools</a> •
  <a href="#themes">Themes</a> •
  <a href="#development">Development</a>
</p>

---

## Overview

Ticca is a sleek, native desktop application for AI-assisted coding built with Rust and the [Iced](https://iced.rs) GUI framework. Unlike browser-based alternatives, Ticca runs locally with direct filesystem access, enabling AI agents to read, write, and execute code in your actual development environment.

### Why Ticca?

- **🔒 Privacy First** — Your code stays on your machine. No uploads to third-party servers.
- **⚡ Native Performance** — Built in Rust for speed and low resource usage.
- **🔧 Real Filesystem Access** — AI agents can read, write, edit, and execute files directly.
- **🤖 Multi-Agent Architecture** — Specialized agents for planning, coding, exploration, and skills.
- **🎨 Beautiful UI** — 11 built-in themes including Catppuccin, Dracula, Nord, and more.
- **🔑 Zero API Keys Required** — OAuth authentication for Claude, ChatGPT, and Gemini.

---

## Features

### 🤖 Multi-Agent System

Ticca employs specialized AI agents, each optimized for specific tasks:

| Agent | Purpose | Access Level |
|-------|---------|--------------|
| **Coding** | Code generation, modification, and execution | Full Access |
| **Planning** | Strategic task breakdown and roadmap creation | Read-Only |
| **Explore** | Fast codebase navigation and search | Read-Only |
| **Skills** | Python-based skill execution for specialized tasks | Full Access |

Agents can invoke each other, enabling complex workflows like: Planning → Exploration → Implementation → Validation.

### 🛠️ Native Tool Integration

All tools are implemented in Rust for maximum performance:

**File Operations**
- `list_files` — Directory listing with intelligent filtering (ignores build artifacts, node_modules, etc.)
- `read_file` — Read files with optional line-range selection
- `edit_file` — Surgical text replacements with diff generation
- `write_file` — Create new files
- `delete_file` — Remove files with safety confirmations

**Search & Analysis**
- `grep` — Ripgrep-powered regex search across your entire codebase

**System Execution**
- `execute_shell` — Run commands in integrated terminal instances
- `list_processes` — View active terminal processes
- `read_process_output` — Stream process output
- `kill_process` — Terminate running processes

**Agent Coordination**
- `invoke_agent` — Cross-agent communication with isolated message histories
- `todo_read` / `todo_write` — Agent-scoped task management

### 🎨 Theme System

11 beautiful themes with instant switching:

| Dark Themes | Light Themes |
|-------------|--------------|
| Dark (default) | Light |
| Dracula | Catppuccin Latte |
| Nord | Gruvbox Light |
| Catppuccin Mocha | |
| Tokyo Night | |
| One Dark | |
| Gruvbox Dark | |
| Zinc | |

### 🔌 MCP Support

Connect external tools via the [Model Context Protocol](https://modelcontextprotocol.io/). Configure MCP servers in settings to extend Ticca's capabilities with custom tools.

### 📦 External Tool Management

On-demand downloads for specialized tasks:
- **UV** — Fast Python package manager
- **Pandoc** — Document conversion
- **Node.js** — JavaScript runtime
- **LibreOffice** — Document processing (AppImage)

---

## Installation

### Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/jan/ticca-desktop/releases).

| Platform | Download |
|----------|----------|
| Linux (x64) | `ticca-linux-x86_64.tar.gz` |
| macOS (Apple Silicon) | `ticca-macos-aarch64.dmg` |
| macOS (Intel) | `ticca-macos-x86_64.dmg` |
| Windows | `ticca-windows-x86_64.zip` |

### Build from Source

**Prerequisites:**
- Rust 1.75+ with Cargo
- System dependencies for Iced (see [Iced setup guide](https://book.iced.rs/setup.html))

```bash
# Clone the repository
git clone https://github.com/jan/ticca-desktop.git
cd ticca-desktop

# Build in release mode
cargo build --release

# Run the application
./target/release/ticca-app
```

**Linux Dependencies (Debian/Ubuntu):**
```bash
sudo apt install libxkbcommon-dev libwayland-dev libvulkan-dev
```

**Linux Dependencies (Fedora):**
```bash
sudo dnf install libxkbcommon-devel wayland-devel vulkan-loader-devel
```

---

## Quick Start

### 1. Launch Ticca

```bash
cargo run --release -p ticca-app
```

Or run the pre-built binary.

### 2. Connect an AI Provider

Click the settings icon (⚙️) and authenticate with one of:
- **Claude** — Anthropic's Claude (recommended)
- **ChatGPT** — OpenAI's GPT models
- **Gemini** — Google's Gemini models

OAuth flows handle authentication automatically — no API keys required!

### 3. Open a Project

Use `Ctrl+O` to open a project directory, or Ticca will use the current working directory.

### 4. Start Coding

Example prompts to try:

```
Explore this codebase and explain the architecture
```

```
Add input validation to the user registration form
```

```
Write tests for the authentication module
```

```
Refactor the database queries to use prepared statements
```

---

## AI Providers

### OAuth Authentication (Recommended)

Ticca supports OAuth PKCE flows for major AI providers:

| Provider | Models | Notes |
|----------|--------|-------|
| **Claude** | claude-sonnet-4, claude-opus-4, claude-3.5-sonnet | Full feature support |
| **ChatGPT** | gpt-4o, gpt-4-turbo, gpt-3.5-turbo | Uses Codex backend |
| **Gemini** | gemini-2.0-flash, gemini-1.5-pro | *(Deprecated)* |

### API Key Authentication

For OpenAI-compatible providers, you can also use API keys:

- OpenAI
- Groq
- Mistral
- Together AI
- Any OpenAI-compatible endpoint

Configure API keys in Settings → Providers.

---

## Agents

### Coding Agent

The primary agent for code implementation. Has full filesystem access and can:
- Read and analyze existing code
- Write new files and modify existing ones
- Execute shell commands
- Run tests and build scripts
- Invoke other agents for specialized tasks

**Workflow:** Reason → Execute → Validate (iterative cycle)

### Planning Agent

Strategic planning with read-only access. Perfect for:
- Breaking down complex features into tasks
- Analyzing project architecture
- Creating implementation roadmaps
- Coordinating multi-agent workflows

**Workflow:** Project Analysis → Requirement Deconstruction → Technical Specification → Agent Coordination

### Explore Agent

Fast, read-only exploration optimized for:
- Finding files by patterns
- Searching code with regex
- Understanding project structure
- Locating implementations and usages

Use multiple Explore agents in parallel for faster discovery.

### Skills Agent

Executes Python-based skills for specialized tasks:
- Document conversion
- Data processing
- Custom automation scripts

Skills are discovered automatically from the `skills/` directory.

---

## Tools

### File Operations

```
list_files          List directory contents with smart filtering
read_file           Read file with optional line range
edit_file           Replace text with surgical precision
write_file          Create new files
delete_file         Remove files with confirmation
```

### Search

```
grep                Ripgrep-powered regex search
```

### System Execution

```
execute_shell       Run shell commands in UI terminal
list_processes      List active terminal processes
read_process_output Read new output from a process
kill_process        Terminate a running process
```

### Agent Coordination

```
list_agents         Discover available agents
invoke_agent        Call another agent with isolated history
todo_read           Read agent's task list
todo_write          Update agent's task list
```

---

## Themes

Switch themes instantly with `Ctrl+T` or through Settings.

### Dark Themes
- **Dark** — Clean modern dark theme (default)
- **Dracula** — Popular purple-accented theme
- **Nord** — Arctic, bluish color palette
- **Catppuccin Mocha** — Warm, cozy dark theme
- **Tokyo Night** — Inspired by Tokyo's night lights
- **One Dark** — Atom's iconic dark theme
- **Gruvbox Dark** — Retro groove colors
- **Zinc** — Minimal zinc-based palette

### Light Themes
- **Light** — Clean modern light theme
- **Catppuccin Latte** — Warm, cozy light theme
- **Gruvbox Light** — Retro groove colors, light variant

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New conversation |
| `Ctrl+O` | Open project directory |
| `Ctrl+T` | Cycle themes |
| `Ctrl+,` | Open settings |
| `Ctrl+Enter` | Send message |
| `Escape` | Cancel current operation |
| `Ctrl+L` | Clear conversation |

---

## Project Architecture

```
ticca-desktop/
├── crates/
│   ├── ticca-app/          # Desktop GUI application
│   │   ├── src/
│   │   │   ├── app/        # Main application state
│   │   │   ├── views/      # UI components
│   │   │   ├── theme/      # 11 built-in themes
│   │   │   └── widgets/    # Custom Iced widgets
│   │   └── assets/         # Icons and fonts
│   │
│   ├── ticca-core/         # Business logic library
│   │   └── src/
│   │       ├── agents/     # AI agent implementations
│   │       ├── tools/      # Native Rust tools
│   │       ├── llm/        # LLM provider integrations
│   │       ├── config/     # SQLite configuration
│   │       └── session/    # Chat history persistence
│   │
│   └── ticca-oauth/        # OAuth authentication
│       └── src/
│           ├── claude.rs   # Anthropic OAuth
│           ├── chatgpt.rs  # OpenAI OAuth
│           └── gemini.rs   # Google OAuth
│
├── vendor/                 # Vendored dependencies
├── scripts/                # Build and utility scripts
└── docs/                   # Additional documentation
```

---

## Development

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Platform-specific dependencies (see Installation)

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run with logging
RUST_LOG=info cargo run -p ticca-app
```

### Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p ticca-core

# Run with output
cargo test -- --nocapture
```

### Code Style

- Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Use `rustfmt` for formatting
- Run `cargo clippy` for linting
- Keep files under 600 lines; refactor if larger

### Adding a New Theme

1. Create a new file in `crates/ticca-app/src/theme/` (e.g., `my_theme.rs`)
2. Define color constants and implement `pub fn theme() -> Theme`
3. Add the variant to `AppTheme` enum in `mod.rs`
4. Add to `ALL_THEMES` array and implement `Display`/`FromStr`

### Adding a New Tool

1. Create the tool in `crates/ticca-core/src/tools/`
2. Implement the `serdes_ai_tools::Tool` trait
3. Register in `ToolRegistry::default()` in `mod.rs`
4. Add to appropriate agent tool sets

---

## Configuration

Configuration is stored in platform-specific directories:

| Platform | Location |
|----------|----------|
| Linux | `~/.local/share/ticca/` |
| macOS | `~/Library/Application Support/ticca/` |
| Windows | `%APPDATA%\ticca\` |

### Files

- `config.db` — SQLite database for settings and tokens
- `sessions/` — Chat history persistence
- `skills/` — Python skills directory

---

## Troubleshooting

### OAuth Login Issues

If the OAuth callback fails:
1. Ensure no firewall is blocking localhost ports 8000-8010
2. Try closing other applications that might use these ports
3. Check the logs with `RUST_LOG=debug cargo run -p ticca-app`

### Linux: FUSE for LibreOffice

LibreOffice is distributed as an AppImage requiring FUSE 2:

```bash
# Debian/Ubuntu
sudo apt install libfuse2

# Fedora
sudo dnf install fuse
```

See [docs/fuse-installation.md](docs/fuse-installation.md) for details.

### Performance Issues

- Ensure you're running a release build (`cargo build --release`)
- Close unused terminal processes in the System Executions tab
- Large projects may benefit from `.ticcaignore` file (same format as `.gitignore`)

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Guidelines

- Follow the existing code style
- Add tests for new functionality
- Update documentation as needed
- Keep commits focused and atomic

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- [Iced](https://iced.rs) — Cross-platform GUI framework
- [serdesAI](https://github.com/michaelpfaffenberger/serdesAI) — LLM agent framework
- [ripgrep](https://github.com/BurntSushi/ripgrep) — Fast regex search
- Theme inspirations: Catppuccin, Dracula, Nord, Tokyo Night, One Dark, Gruvbox

---

<p align="center">
  Made with ❤️ and 🦀
</p>
