# Repository Guidelines

## Project Structure & Module Organization
- `crates/ticca-app/`: Iced GUI application (views, theme, app state, UI helpers).
- `crates/ticca-core/`: Business logic (agents, tools, config/session DB, LLM providers).
- `crates/ticca-oauth/`: OAuth PKCE flows and callback server for providers.
- `vendor/rig-core/`: Patched dependency for rig; avoid edits unless required.
- Assets live in `crates/ticca-app/assets/` (fonts, icons).

## Build, Test, and Development Commands
- `cargo build`: Build the workspace.
- `cargo run -p ticca-app`: Run the desktop app locally.
- `cargo test`: Run all tests.
- `cargo test -p ticca-core`: Run tests for core logic only.
Use `RUST_LOG=info cargo run -p ticca-app` for runtime logs.

## Coding Style & Naming Conventions
- Rust 2024 edition with standard `rustfmt` style (4 spaces, trailing commas).
- Modules are snake_case; types are PascalCase.
- Prefer small, focused modules; keep UI logic in `ticca-app` and domain logic in `ticca-core`.
- Use `anyhow`/`thiserror` for errors and `tracing` for logging.

## Testing Guidelines
- Tests use Rust’s built-in test framework (`#[test]`, `#[tokio::test]`).
- Keep tests near the implementation module.
- Favor small unit tests for tools, agents, and providers.

## Commit & Pull Request Guidelines
- Commit messages follow a lightweight Conventional Commits style: `feat:`, `fix:`, `refactor:`, `ci:`, and release tags like `Release 0.3.x`.
- PRs should include a short summary, key changes, and testing notes.
- UI changes should include screenshots or a brief GIF.
- Link related issues when available.

## Configuration & Security Notes
- OAuth tokens and settings are stored in platform data dirs:
  - Linux: `~/.local/share/ticca-desktop/`
  - macOS: `~/Library/Application Support/ticca-desktop/`
  - Windows: `C:\Users\<User>\AppData\Roaming\ticca-desktop\`
- Avoid committing secrets; use OAuth flows via the Settings UI.
