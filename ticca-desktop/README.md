# Ticca Desktop

A sleek, Iced-based desktop application for AI-assisted coding.

![Ticca Desktop](assets/screenshot.png)

## Features

- 🐶 **Coding Agent**: Your loyal coding companion for writing and modifying code
- 📋 **Planning Agent**: Strategic planning for complex tasks
- 🔐 **OAuth Integration**: Claude, Gemini, and ChatGPT authentication
- 🎨 **Theme Support**: Dark and light modes
- ⌨️ **Keyboard Shortcuts**: Fast navigation and actions
- 💾 **Session Storage**: SQLite-based conversation history
- 🔧 **Native Rust Tools**: File operations, grep (ripgrep), shell commands

## Prerequisites

- Rust 2024 Edition (1.85+)
- Python 3.10+ (for PydanticAI integration)
- Linux with Wayland/Hyprland (primary target)

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

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Send message |
| `Ctrl+N` | New session |
| `Ctrl+,` | Open settings |
| `Ctrl+T` | Toggle theme |
| `Escape` | Close panel |
| `Ctrl+1` | Switch to Coding agent |
| `Ctrl+2` | Switch to Planning agent |

## Project Structure

```
ticca-desktop/
├── crates/
│   ├── ticca-app/           # Iced GUI application
│   ├── ticca-core/          # Business logic, agents, tools
│   ├── ticca-oauth/         # OAuth implementations
│   └── ticca-python-bridge/ # PyO3 bridge to PydanticAI
└── python/                  # Python package for AI integration
```

## Configuration

Configuration is stored in `~/.local/share/ticca-desktop/`:

- `config.db` - Settings, model configs, OAuth tokens
- `sessions.db` - Chat session history

## Available Tools

The agents have access to these native Rust tools:

| Tool | Description |
|------|-------------|
| `list_files` | Directory listing with smart ignore patterns |
| `read_file` | Read files with optional line ranges |
| `edit_file` | Create, modify, or delete file content |
| `delete_file` | Remove files |
| `grep` | Search files using ripgrep |
| `run_shell_command` | Execute shell commands |
| `share_your_reasoning` | Agent thought process sharing |

## Agents

### Coding Agent 🐶

Your loyal coding companion that:
- Writes and modifies code using best practices
- Follows DRY, YAGNI, and SOLID principles
- Keeps files under 600 lines
- Runs tests and validates changes

### Planning Agent 📋

Strategic planner that:
- Analyzes project requirements
- Creates execution roadmaps
- Identifies risks and alternatives
- Explores codebase structure

## OAuth Setup

### Claude (Anthropic)
1. Click "🔐 Claude OAuth" in settings
2. Authorize in your browser
3. Tokens are stored securely in the config database

### Gemini (Google)
1. Requires Google Cloud project with OAuth credentials
2. Click "🔐 Gemini OAuth" in settings
3. Complete Google sign-in

### ChatGPT (OpenAI)
1. Click "🔐 ChatGPT OAuth" in settings
2. Authorize with your OpenAI account

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check all crates
cargo check --all

# Build Python bridge (requires maturin)
cd crates/ticca-python-bridge
maturin develop
```

## Architecture

```
┌───────────────────────────────────────────────────────────┐
│                    Ticca Desktop (Iced UI)                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Chat View   │  │ Config View │  │ OAuth (System Brw) │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├───────────────────────────────────────────────────────────┤
│                        ticca-core                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────┐  │
│  │ Agents   │  │  Tools   │  │  Config  │  │ Session     │  │
│  │(Planning,│  │(File,Grep│  │ (SQLite) │  │ Storage     │  │
│  │ Coding)  │  │ Shell)   │  │          │  │ (SQLite)    │  │
│  └──────────┘  └──────────┘  └──────────┘  └─────────────┘  │
├───────────────────────────────────────────────────────────┤
│                 ticca-python-bridge (PyO3)                    │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  PydanticAI Agent Runner | Streaming Response Handler   │  │
│  └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
```

## Test Coverage

| Crate | Tests |
|-------|-------|
| ticca-core | 40 |
| ticca-oauth | 13 |
| **Total** | **53** |

## License

MIT

## Credits

Built with:
- [Iced](https://github.com/iced-rs/iced) - Cross-platform GUI framework
- [PydanticAI](https://github.com/pydantic/pydantic-ai) - AI agent framework
- [ripgrep](https://github.com/BurntSushi/ripgrep) - Fast text search
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [PyO3](https://github.com/PyO3/pyo3) - Rust-Python bindings
