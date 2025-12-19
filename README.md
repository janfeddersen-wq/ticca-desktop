# Ticca Desktop

A sleek, Iced-based desktop application for AI-assisted coding. Ticca provides a native GUI for interacting with Claude and other LLM providers, with built-in tools for file operations, code search, and shell commands.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)
![Version](https://img.shields.io/badge/version-0.7.0-green.svg)

## Features

### 🤖 AI Agents
- **Coding Agent** - Writes, modifies, and executes code with full tool access
- **Planning Agent** - Breaks down complex tasks into actionable steps and creates execution roadmaps
- **Agent Invocation** - Agents can invoke other agents via the `invoke_agent` tool for multi-agent workflows

### 🛠️ Native Tools
All tools are implemented in Rust for maximum performance:
- **list_files** - List files and directories with intelligent filtering
- **read_file** - Read file contents with optional line range
- **write_file** - Write content to files
- **edit_file** - Edit files using content replacement, text replacements, or snippet deletion
- **delete_file** - Delete files with diff generation
- **grep** - Search for text patterns using ripgrep libraries
- **shell** - Execute shell commands with timeout support
- **list_agents** - List all available agents with their identifiers and descriptions
- **invoke_agent** - Invoke another agent with its own isolated message history

### 🎨 Theming
11 built-in themes including:
- Dark / Light
- Zinc
- Dracula
- Nord
- Catppuccin (Mocha & Latte)
- Tokyo Night
- One Dark
- Gruvbox (Dark & Light)

### 🔐 OAuth Authentication
Native OAuth support for multiple providers:
- **Claude** (Anthropic) - Public PKCE flow
- **Gemini** (Google) - Public PKCE flow (using gemini-cli credentials)
- **Gemini Code Assist** (Google) - Enterprise code assistance provider
- **ChatGPT** (OpenAI) - Public PKCE flow

### 💾 Session Management
- Persistent chat sessions stored in SQLite
- Session history and message recall
- Configuration persistence
- Per-agent model pinning

### 📋 Clipboard & Image Support
- Paste images from clipboard (`Ctrl+V`)
- Drag & drop image files (X11)
- Image attachments in chat messages

## Project Structure

```
ticca-desktop/
├── crates/
│   ├── ticca-app/          # Iced GUI application
│   │   ├── src/
│   │   │   ├── app.rs           # Main application state
│   │   │   ├── views/           # UI views (chat, config)
│   │   │   ├── theme/           # Theme definitions (11 themes)
│   │   │   ├── llm_stream.rs    # Streaming LLM responses
│   │   │   ├── keybindings.rs   # Keyboard shortcuts
│   │   │   ├── session_manager.rs # Session persistence
│   │   │   ├── image_handler.rs # Image clipboard/drag-drop
│   │   │   ├── agent_graph.rs   # Agent invocation graph visualization
│   │   │   └── ...
│   │   └── assets/
│   │       └── fonts/           # Noto Sans font family
│   │
│   ├── ticca-core/         # Core business logic
│   │   └── src/
│   │       ├── agents/          # AI agent definitions
│   │       ├── tools/           # Native tool implementations
│   │       │   ├── rig_tools.rs # Rig-compatible tool wrappers
│   │       │   ├── file_ops.rs  # list_files, read_file
│   │       │   ├── file_mods.rs # edit_file, delete_file
│   │       │   ├── grep.rs      # Text search
│   │       │   └── shell.rs     # Shell command execution
│   │       ├── config/          # SQLite configuration database
│   │       ├── session/         # Session storage
│   │       └── llm/             # LLM providers and clients
│   │           ├── model_service.rs    # Model selection and management
│   │           ├── provider_registry.rs # LLM provider registry
│   │           └── providers/          # Claude, Gemini, ChatGPT providers
│   │
│   └── ticca-oauth/        # OAuth implementations
│       └── src/
│           ├── claude.rs        # Claude/Anthropic OAuth
│           ├── gemini.rs        # Google Gemini OAuth
│           ├── chatgpt.rs       # OpenAI ChatGPT OAuth
│           ├── pkce.rs          # PKCE utilities
│           └── callback_server.rs # Local OAuth callback server
│
├── vendor/
│   └── rig-core/          # Patched rig-core dependency
│
├── Cargo.toml              # Workspace configuration
├── LICENSE                 # MIT License
├── AGENTS.md               # Agent configuration guidance
└── README.md
```

## Requirements

- **Rust** 2024 edition (1.85+)
- **Linux/macOS/Windows** - Cross-platform support via Iced
- **SQLite** - Bundled via rusqlite

### Linux Dependencies

On Linux, you may need to install additional packages for the GUI:

```bash
# Ubuntu/Debian
sudo apt install libxkbcommon-dev libwayland-dev

# Fedora
sudo dnf install libxkbcommon-devel wayland-devel
```

## Building

```bash
# Clone the repository
git clone https://github.com/jan/ticca-desktop
cd ticca-desktop

# Build in release mode
cargo build --release

# Run the application
cargo run --release
```

The compiled binary will be at `target/release/ticca`.

## Configuration

Ticca stores its configuration in a platform-specific data directory:

- **Linux**: `~/.local/share/ticca-desktop/`
- **macOS**: `~/Library/Application Support/ticca-desktop/`
- **Windows**: `C:\Users\<User>\AppData\Roaming\ticca-desktop\`

Configuration includes:
- `config.db` - Settings, model configurations, OAuth tokens
- `sessions.db` - Chat session history

## Usage

### Starting a Chat

1. Launch Ticca Desktop
2. Authenticate with your preferred LLM provider (Claude, etc.)
3. Select an agent (Coding or Planning)
4. Start chatting!

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Send message |
| `Ctrl+N` | New session |
| `Ctrl+,` | Open settings |
| `Ctrl+T` | Toggle theme |
| `Ctrl+1` | Switch to Coding agent |
| `Ctrl+2` | Switch to Planning agent |
| `Ctrl+V` | Paste (including images) |
| `Escape` | Close panel |

### Working with Tools

The Coding Agent has access to all native tools. Example prompts:

```
"List all Rust files in the src directory"
"Read the contents of Cargo.toml"
"Search for 'TODO' comments in the codebase"
"Run cargo test"
```

## Architecture

### Crate Overview

| Crate | Purpose |
|-------|---------|
| `ticca-app` | Iced GUI application, views, theming |
| `ticca-core` | Business logic, agents, tools, database |
| `ticca-oauth` | OAuth flows for LLM providers |

### Key Dependencies

- **[Iced](https://iced.rs/)** - Cross-platform GUI framework (v0.14)
- **[rig-core](https://github.com/0xPlaygrounds/rig)** - LLM framework with tool support (v0.27)
- **[rusqlite](https://github.com/rusqlite/rusqlite)** - SQLite bindings
- **[tokio](https://tokio.rs/)** - Async runtime
- **[ripgrep libraries](https://github.com/BurntSushi/ripgrep)** - Fast text search (grep-regex, grep-searcher, ignore)
- **[arboard](https://github.com/1Password/arboard)** - Cross-platform clipboard with image support
- **[twemoji-assets](https://crates.io/crates/twemoji-assets)** - Emoji rendering via Twemoji SVGs

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p ticca-core
```

### Logging

Set the `RUST_LOG` environment variable to control log output:

```bash
RUST_LOG=info cargo run
RUST_LOG=debug cargo run
RUST_LOG=ticca_core=debug cargo run
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Copyright (c) 2025 Jan

---

*Ticca Desktop - Your AI coding companion*
