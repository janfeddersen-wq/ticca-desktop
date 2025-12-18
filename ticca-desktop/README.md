# Ticca Desktop

<p align="center">
  <strong>A modern, cross-platform desktop application for AI-assisted coding</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/GUI-Iced_0.14-blue?logo=rust" alt="Iced">
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License">
  <img src="https://img.shields.io/badge/Tests-58_passing-brightgreen" alt="Tests">
</p>

---

## Overview

Ticca Desktop is a **100% Rust** native desktop application that brings AI-powered coding assistance directly to your desktop. No Electron, no JavaScript—just pure Rust performance with a beautiful, responsive GUI built on the [Iced](https://github.com/iced-rs/iced) framework.

### Why Ticca?

- **🚀 Native Performance** — Lightweight and fast, with minimal resource usage
- **🔒 Privacy First** — Your code stays local; only what you explicitly share goes to the LLM
- **⌨️ Keyboard-Centric** — Designed for developers who prefer keyboard navigation
- **🔐 OAuth Authentication** — No API keys to manage; authenticate directly with providers
- **📎 Image Support** — Attach images and screenshots for vision model analysis

---

## ✨ Features

### 🤖 Intelligent Agents

| Agent | Description | Tools |
|-------|-------------|-------|
| **Coding Agent** | Your hands-on coding partner. Reads, writes, and modifies files; executes shell commands; follows DRY, YAGNI, and SOLID principles. | `list_files`, `read_file`, `grep`, `edit_file`, `write_file`, `shell` |
| **Planning Agent** | Strategic task breakdown. Analyzes project structure, creates execution roadmaps, and identifies dependencies. Read-only access for safe exploration. | `list_files`, `read_file`, `grep` |

### 🔧 Native Tools

All tools are implemented in pure Rust with no external dependencies:

| Tool | Description |
|------|-------------|
| `list_files` | Explore project structure with intelligent `.gitignore` filtering |
| `read_file` | Read file contents with optional line ranges for large files |
| `edit_file` | Precise text replacement in existing files (requires exact match) |
| `write_file` | Create new files or overwrite existing ones |
| `delete_file` | Remove files with diff generation for review |
| `grep` | Fast regex search powered by [ripgrep](https://github.com/BurntSushi/ripgrep) libraries |
| `shell` | Execute shell commands with configurable timeout protection |

### 🔐 OAuth Authentication

Native OAuth flows for major LLM providers—no API keys to manage:

| Provider | Flow Type | Status |
|----------|-----------|--------|
| **Claude** (Anthropic) | Public PKCE | ✅ Implemented |
| **Gemini** (Google) | OAuth with client credentials | ✅ Implemented |
| **ChatGPT** (OpenAI) | Public PKCE | ✅ Implemented |

### 💾 Session Persistence

- SQLite-backed conversation history
- Automatic session management with timestamps
- Token usage tracking per session
- Full message history with role preservation (user, assistant, system, tool)

### 🎨 Rich Markdown Rendering

- Full markdown support with syntax highlighting (powered by [syntect](https://github.com/trishume/syntect))
- Emoji rendering via Twemoji SVG
- Math notation support (superscripts, subscripts, Unicode math symbols)
- Table rendering
- Copy code blocks with one click

### 🖼️ Image Attachments

- Attach images to messages for vision model analysis
- Paste images directly from clipboard
- Support for PNG, JPEG, GIF, and WebP formats

### 🎨 Themes

Eleven built-in themes to match your style:

| Theme | Description |
|-------|-------------|
| **Dark** | Easy on the eyes for long coding sessions (default) |
| **Light** | Clean and bright for well-lit environments |
| **Zinc** | Neutral gray tones for a minimal aesthetic |
| **Dracula** | Popular dark theme with vibrant colors |
| **Nord** | Arctic, north-bluish color palette |
| **Catppuccin Mocha** | Soothing pastel theme (dark) |
| **Catppuccin Latte** | Soothing pastel theme (light) |
| **Tokyo Night** | Dark theme inspired by Tokyo city lights |
| **One Dark** | Atom's iconic dark theme |
| **Gruvbox Dark** | Retro groove color scheme (dark) |
| **Gruvbox Light** | Retro groove color scheme (light) |

### ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Send message |
| `Ctrl+N` | New session |
| `Ctrl+,` | Open settings |
| `Ctrl+T` | Toggle theme |
| `Ctrl+1` | Switch to Coding agent |
| `Ctrl+2` | Switch to Planning agent |
| `Escape` | Close panel |

---

## 📦 Installation

### Prerequisites

- **Rust 2024 Edition** (1.85+)
- **Linux**: Development libraries for your display server
  ```bash
  # Wayland
  sudo apt install libwayland-dev libxkbcommon-dev
  
  # X11
  sudo apt install libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev
  ```
- **macOS/Windows**: Should work out of the box

### Build from Source

```bash
# Clone the repository
git clone https://github.com/jan/ticca-desktop.git
cd ticca-desktop

# Build in release mode (recommended)
cargo build --release

# Run
./target/release/ticca
```

### Development

```bash
# Run in debug mode with logging
RUST_LOG=info cargo run

# Run all tests (58 tests across 3 crates)
cargo test

# Run tests for a specific crate
cargo test -p ticca-core
cargo test -p ticca-oauth
cargo test -p ticca-app

# Run with verbose output
cargo test -- --nocapture
```

---

## 🏗️ Architecture

Ticca is organized as a Cargo workspace with three crates:

```
ticca-desktop/
├── crates/
│   ├── ticca-app/           # 🖥️ Iced GUI application
│   │   ├── src/
│   │   │   ├── app.rs               # Main application state & update loop
│   │   │   ├── main.rs              # Entry point
│   │   │   ├── llm_stream.rs        # Streaming response handling
│   │   │   ├── keybindings.rs       # Keyboard shortcuts
│   │   │   ├── messages.rs          # Application messages/events
│   │   │   ├── icons.rs             # Icon definitions
│   │   │   ├── material_icons.rs    # Material Icons integration
│   │   │   ├── image_handler.rs     # Image attachment processing
│   │   │   ├── theme/               # 11 color themes
│   │   │   │   ├── dark.rs          # Default dark theme
│   │   │   │   ├── light.rs         # Light theme
│   │   │   │   ├── zinc.rs          # Zinc neutral theme
│   │   │   │   ├── dracula.rs       # Dracula theme
│   │   │   │   ├── nord.rs          # Nord theme
│   │   │   │   ├── catppuccin_mocha.rs
│   │   │   │   ├── catppuccin_latte.rs
│   │   │   │   ├── tokyo_night.rs
│   │   │   │   ├── one_dark.rs
│   │   │   │   ├── gruvbox_dark.rs
│   │   │   │   ├── gruvbox_light.rs
│   │   │   │   └── styles.rs        # Widget styling
│   │   │   └── views/
│   │   │       ├── config.rs        # Settings panel
│   │   │       └── components/      # Reusable UI components
│   │   │           ├── markdown.rs  # Markdown renderer with emoji & math
│   │   │           ├── syntax.rs    # Syntax highlighting
│   │   │           └── emoji.rs     # Twemoji SVG rendering
│   │   └── assets/fonts/            # Noto Sans font family
│   │
│   ├── ticca-core/          # 🧠 Business logic & tools
│   │   └── src/
│   │       ├── agents/              # Agent definitions
│   │       │   ├── base.rs          # Agent trait & types
│   │       │   ├── coding.rs        # Coding agent (full tools)
│   │       │   └── planning.rs      # Planning agent (read-only)
│   │       ├── tools/               # Native Rust tools
│   │       │   ├── registry.rs      # Tool registration system
│   │       │   ├── file_ops.rs      # list_files, read_file
│   │       │   ├── file_mods.rs     # edit_file, write_file, delete_file
│   │       │   ├── grep.rs          # Regex search (ripgrep)
│   │       │   ├── shell.rs         # Command execution
│   │       │   └── rig_tools.rs     # Rig framework integration
│   │       ├── config/              # Configuration storage
│   │       │   ├── database.rs      # SQLite config DB
│   │       │   ├── models.rs        # Config data models
│   │       │   └── migrations.rs    # Schema migrations
│   │       ├── session/             # Conversation persistence
│   │       │   ├── database.rs      # SQLite session DB
│   │       │   └── models.rs        # Session/Message models
│   │       └── llm/
│   │           └── claude.rs        # Claude API client (streaming)
│   │
│   └── ticca-oauth/         # 🔐 OAuth implementations
│       └── src/
│           ├── common.rs            # Shared types & errors
│           ├── pkce.rs              # PKCE utilities
│           ├── callback_server.rs   # Local OAuth callback server
│           ├── claude.rs            # Anthropic OAuth
│           ├── gemini.rs            # Google OAuth
│           └── chatgpt.rs           # OpenAI OAuth
│
├── Cargo.toml               # Workspace configuration
├── LICENSE                  # MIT License
└── README.md
```

### Crate Dependencies

```
ticca-app
    ├── ticca-core (agents, tools, sessions, LLM client)
    └── ticca-oauth (authentication flows)

ticca-core
    └── ticca-oauth (token management)
```

---

## 🔌 Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| [iced](https://github.com/iced-rs/iced) | 0.14 | Cross-platform GUI framework |
| [tokio](https://tokio.rs) | 1.43 | Async runtime |
| [rusqlite](https://github.com/rusqlite/rusqlite) | 0.32 | SQLite database (bundled) |
| [reqwest](https://github.com/seanmonstar/reqwest) | 0.12 | HTTP client with streaming |
| [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) | 0.11 | Markdown parsing |
| [syntect](https://github.com/trishume/syntect) | 5.2 | Syntax highlighting |
| [grep-regex](https://github.com/BurntSushi/ripgrep) | 0.1 | Ripgrep search engine |
| [twemoji-assets](https://github.com/nickelc/twemoji-assets) | 1.5 | Emoji SVG rendering |
| [arboard](https://github.com/1Password/arboard) | 3.x | Clipboard support (with images) |
| [material-icons](https://github.com/nickelc/material-icons) | 0.2 | Material Design icons |
| [rfd](https://github.com/PolyMeilex/rfd) | 0.15 | Native file dialogs |

---

## 🧪 Testing

The project includes comprehensive tests across all crates:

```bash
# Run all tests
cargo test

# Current test coverage: 58 tests passing
# - ticca-app: 2 tests (keybindings)
# - ticca-core: 42 tests (agents, tools, config, sessions, LLM)
# - ticca-oauth: 13 tests (PKCE, callback server, provider flows)
# - Doc tests: 1 test
```

Test categories:
- **Agent behavior** — Tool availability, system prompts
- **Tool execution** — File operations, grep, shell commands
- **Database CRUD** — Sessions, messages, config, OAuth tokens
- **OAuth flows** — PKCE generation, auth URL building, callback handling

---

## 🛣️ Roadmap

- [ ] Gemini model integration (OAuth ready)
- [ ] ChatGPT model integration (OAuth ready)
- [ ] Session export/import (JSON)
- [ ] Plugin system for custom tools
- [ ] Multi-file diff view
- [ ] Git integration (status, diff, commit)
- [ ] Model selection per session
- [ ] Configurable max tool rounds

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Run tests (`cargo test`)
4. Commit your changes (`git commit -m 'Add amazing feature'`)
5. Push to the branch (`git push origin feature/amazing-feature`)
6. Open a Pull Request

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [Iced](https://github.com/iced-rs/iced) — For the excellent Rust GUI framework
- [Anthropic](https://anthropic.com) — For Claude and the inspiration
- [ripgrep](https://github.com/BurntSushi/ripgrep) — For the grep libraries
- [Noto Fonts](https://fonts.google.com/noto) — For the beautiful font family
- [Twemoji](https://github.com/twitter/twemoji) — For emoji assets
- [Material Design Icons](https://fonts.google.com/icons) — For the icon set
- Theme inspirations: [Dracula](https://draculatheme.com/), [Nord](https://www.nordtheme.com/), [Catppuccin](https://github.com/catppuccin), [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme), [One Dark](https://github.com/atom/atom/tree/master/packages/one-dark-ui), [Gruvbox](https://github.com/morhetz/gruvbox)

---

<p align="center">
  Built with 🦀 Rust and ❤️
</p>
