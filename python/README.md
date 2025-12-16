# Ticca Agent 🐍

AI Agent system for Ticca Desktop built with PydanticAI.

## Overview

Ticca Agent provides a flexible, type-safe AI agent framework for integrating multiple AI providers (OpenAI, Anthropic, local models) into Ticca Desktop.

## Features

- 🔌 **Multi-Provider Support** - Seamlessly switch between AI providers
- 📝 **Full Type Safety** - Pydantic v2 models with mypy strict compliance
- 🔄 **Streaming Responses** - Real-time token streaming for responsive UIs
- 🛠️ **Tool System** - Extensible tool/function calling support
- 🔐 **MCP Integration** - Model Context Protocol for interoperability
- ⚡ **Async-First** - Built on anyio for high-performance async I/O
- 🔁 **Resilient** - Exponential backoff retry with configurable policies

## Installation

```bash
# With pip
pip install -e .

# With development dependencies
pip install -e ".[dev]"
```

## Quick Start

```python
from ticca_agent.core import AgentRequest, Message, MessageRole
from ticca_agent.providers import ProviderConfig

# Create a request
request = AgentRequest(
    messages=[
        Message(role=MessageRole.USER, content="Hello, AI!")
    ]
)

# Configure a provider (example)
config = ProviderConfig(
    api_key="your-api-key",
    timeout=30.0,
)
```

## Project Structure

```
ticca_agent/
├── __init__.py          # Package root with version
├── core/                 # Core types and utilities
│   ├── __init__.py
│   └── types.py         # Pydantic models (Request, Response, Message, etc.)
├── providers/           # AI provider implementations
│   ├── __init__.py
│   └── base.py          # Abstract base provider with retry logic
├── agents/              # Agent implementations
├── tools/               # Tool definitions
├── session/             # Session management
├── mcp/                 # Model Context Protocol integration
└── utils/               # Shared utilities
```

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest --cov

# Type checking
mypy . --strict

# Linting
ruff check .

# Formatting
black .
```

## Requirements

- Python >= 3.11
- pydantic >= 2.0
- pydantic-ai >= 0.1
- httpx >= 0.27
- anyio >= 4.0

## License

MIT
