# Ticca Desktop

🖥️ A high-performance native AI chat application built with Rust/GPUI and Python/PydanticAI.

## Overview

Ticca Desktop is a GPU-accelerated desktop application designed for AI agent interactions,
targeting 120FPS smooth UI performance using GPUI. The application combines:

- **Rust/GPUI Frontend**: Native, GPU-accelerated UI for butter-smooth interactions
- **Python/PydanticAI Backend**: Powerful AI agent capabilities via PyO3 bridge
- **SQLite Persistence**: Local-first data storage for conversations and settings

## Architecture

This is a Rust workspace with the following crates:

| Crate | Type | Description |
|-------|------|-------------|
| `ticca-ui` | binary | GPUI-based frontend application |
| `ticca-core` | lib | Core business logic and domain types |
| `ticca-bridge` | lib | PyO3 bridge for Python interoperability |
| `ticca-db` | lib | SQLite persistence layer via sqlx |
| `ticca-config` | lib | Configuration management |

## Prerequisites

- **Rust**: Stable toolchain (managed via `rust-toolchain.toml`)
- **Python 3.10+**: For PyO3 bridge functionality
- **SQLite**: For local persistence
- **System dependencies for GPUI**:
  - Linux: `libxcb`, `libxkbcommon`, `vulkan-loader`
  - macOS: Xcode command line tools
  - Windows: Visual Studio Build Tools

## Getting Started

### Clone and Build

```bash
git clone https://github.com/ticca/ticca-desktop.git
cd ticca-desktop
cargo build
```

### Run the Application

```bash
cargo run -p ticca-ui
```

### Run Tests

```bash
cargo test --workspace
```

### Check Code Quality

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

## Development

### Project Structure

```
ticca-desktop/
├── Cargo.toml              # Workspace root
├── rust-toolchain.toml     # Rust toolchain configuration
├── crates/
│   ├── ticca-ui/           # Main application binary (GPUI frontend)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   ├── ticca-core/         # Core business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── ticca-bridge/       # PyO3 bridge for Python interoperability
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   ├── ticca-db/           # SQLite persistence layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   └── ticca-config/       # Configuration management
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── python/                  # Python AI agent backend
│   ├── pyproject.toml
│   └── ticca_agent/        # PydanticAI-powered agents
│       ├── agents/
│       ├── core/
│       ├── mcp/
│       ├── providers/
│       ├── session/
│       ├── tools/
│       └── utils/
└── README.md
```

### Coding Standards

- **No `unwrap()`**: Use proper error handling with `?`, `anyhow`, or `thiserror`
- **Async**: All async code uses Tokio runtime
- **UI**: GPUI is the only UI framework - no alternatives
- **Files**: Keep files under 600 lines; split if larger

## License

MIT
