# Ticca Desktop - Business Requirements

**Purpose:** AI-powered code generation agent system with multi-agent support and multi-model orchestration.

---

## Table of Contents

1. [Authentication & API Key Management](#1-authentication--api-key-management)
2. [AI Provider Integration](#2-ai-provider-integration)
3. [Configuration System](#3-configuration-system)
4. [Model Pinning](#4-model-pinning)
5. [Model Settings](#5-model-settings)
6. [Agent System](#6-agent-system)
7. [Tool System](#7-tool-system)
8. [MCP Server Integration](#8-mcp-server-integration)
9. [Plugin System](#9-plugin-system)
10. [Session & Context Management](#10-session--context-management)
11. [System Prompts & Personas](#11-system-prompts--personas)

---

## 1. Authentication & API Key Management

### 1.1 API Key Loading

**Requirement:** Load API keys from multiple sources with priority ordering.


**Supported API Keys:**
| Environment Variable | Provider |
|---------------------|----------|
| `OPENAI_API_KEY` | OpenAI models |
| `ANTHROPIC_API_KEY` | Claude/Anthropic models |
| `GEMINI_API_KEY` | Google Gemini models |
| `CEREBRAS_API_KEY` | Cerebras models |
| `OPENROUTER_API_KEY` | OpenRouter aggregator |
| `ZAI_API_KEY` | Z.ai models |
| `AZURE_OPENAI_API_KEY` | Azure OpenAI |
| `AZURE_OPENAI_ENDPOINT` | Azure endpoint URL |

**Behavior:** Missing keys trigger warnings (not errors) - models skip gracefully and continue loading other providers.

### 1.2 OAuth Authentication

#### Claude Code OAuth

**Configuration Values:**
```
Auth URL:         https://claude.ai/oauth/authorize
Token URL:        https://console.anthropic.com/v1/oauth/token
API Base URL:     https://api.anthropic.com
Client ID:        9d1c250a-e61b-44d9-88ed-5944d1962f5e
Scope:            org:create_api_key user:profile user:inference
Redirect Host:    http://localhost
Redirect Path:    callback
Port Range:       8765-8795
Callback Timeout: 180 seconds
Anthropic Version: 2023-06-01
Model Prefix:     claude-code-
Default Context:  200000 tokens
API Key Env Var:  CLAUDE_CODE_ACCESS_TOKEN
```

**Flow:** PKCE-based OAuth 2.0 with state validation
- Start local HTTP callback server on available port
- Open browser to authorization endpoint
- Exchange authorization code for tokens
- Auto-discover available Claude models after authentication
- Store access_token, refresh_token, id_token, expires_in

#### ChatGPT OAuth (Currently Disabled)

**Configuration Values:**
```
Issuer:           https://auth.openai.com
Token URL:        https://auth.openai.com/oauth/token
Client ID:        app_EMoamEEZ73f0CkXaXp7hrann
Required Port:    1455
Callback Timeout: 120 seconds
Session Expiry:   4 minutes (300 seconds)
```

### 1.3 Custom Endpoint Authentication

Models can use custom endpoints with:
- Environment variable references using `$` prefix (e.g., `$CUSTOM_API_KEY`)
- Direct API key values in config
- Custom headers with environment variable expansion
- Resolution happens at model creation time, not config load time

---

## 2. AI Provider Integration

### 2.1 Supported Providers

| Provider | Type | Notes |
|----------|------|-------|
| Anthropic/Claude | Direct | All Claude models (Haiku, Sonnet, Opus 4.5) |
| OpenAI | Direct | GPT models including GPT-5.1 |
| Azure OpenAI | Direct | Azure-hosted OpenAI models |
| Google Gemini | Direct | Standard generative models |
| Gemini OAuth | OAuth | Code Assist API (cloudcode-pa.googleapis.com) |
| OpenRouter | Aggregator | Access to 65+ models |
| Cerebras | Direct | High-speed inference |
| Z.AI | Custom | Coding and general API endpoints |
| Custom endpoints | Flexible | OpenAI/Anthropic/Gemini compatible |

### 2.2 HTTP Client Requirements

**Retry Transport:**
- Retryable HTTP status codes: 429, 502, 503, 504
- Maximum retry attempts: 10
- Wait strategy: Exponential backoff with `Retry-After` header awareness
- Fallback wait: Exponential (multiplier=1, max=60 seconds)
- Maximum wait time: 300 seconds

**Timeout:** Default 180 seconds per request

**HTTP/2:** Optional, configurable (default: disabled)

### 2.3 Anthropic Cache Control Injection

**Requirement:** Intercept requests to Anthropic's `/v1/messages` endpoint and inject cache control headers on the last message content block.

**Header to inject:**
```json
{"cache_control": {"type": "ephemeral"}}
```

This enables prompt caching for reduced latency and costs.

### 2.4 Claude-Code Model Special Handling

**CRITICAL REQUIREMENT:** Claude-code models require special prompt handling because the endpoint does not support custom system prompts.

**Detection:** Model name starts with `"claude-code"`

**Required Instruction Override String:**
```
"You are Claude Code, Anthropic's official CLI for Claude."
```

**Prompt Rewriting Process:**
1. The original system prompt is **prepended to the user's first message**
2. The agent's `instructions` field is **replaced** with the fixed string above
3. This happens transparently before each API call

**Example Transformation:**
```
BEFORE:
  instructions: "You are a helpful coding assistant."
  user_prompt: "Write a hello world program"

AFTER:
  instructions: "You are Claude Code, Anthropic's official CLI for Claude."
  user_prompt: "You are a helpful coding assistant.\n\nWrite a hello world program"
```

### 2.5 Model Types

| Type | Description |
|------|-------------|
| `gemini` | Google models |
| `openai` | OpenAI models |
| `anthropic` | Anthropic models with cache injection |
| `custom_anthropic` | Custom Anthropic-compatible endpoints |
| `claude_code` | Claude Code endpoint (uses auth_token instead of api_key) |
| `azure_openai` | Azure OpenAI |
| `custom_openai` | Custom OpenAI-compatible endpoints |
| `zai_coding` / `zai_api` | Z.AI endpoints |
| `custom_gemini` | Custom Gemini-compatible endpoints |
| `cerebras` | Cerebras inference |
| `openrouter` | OpenRouter aggregation |
| `gemini_oauth` | OAuth-based Gemini Code Assist |
| `round_robin` | Load balancing across multiple models |

---

## 3. Configuration System

### 3.1 Configuration Location

**Application Directory:** `~/.ticca_desktop/`

### 3.2 Configuration Keys

**Core Settings:**
| Key | Description | Default |
|-----|-------------|---------|
| `model` | Global default model | First in models config |
| `temperature` | Global temperature | None (model default) |
| `default_agent` | Startup agent | "code-puppy" |

**Message/Context Settings:**
| Key | Default | Description |
|-----|---------|-------------|
| `protected_token_count` | 50000 | Tokens to preserve during compaction |
| `compaction_strategy` | "truncation" | "summarization" or "truncation" |
| `compaction_threshold` | 0.85 | Context usage trigger (0.0-1.0) |
| `message_limit` | 1000 | Max requests per session |
| `auto_save_session` | true | Auto-save after responses |
| `max_saved_sessions` | 20 | Number of sessions to keep |

**Display Settings:**
| Key | Default | Description |
|-----|---------|-------------|
| `diff_context_lines` | 6 | Context lines in diffs (0-50) |
| `suppress_thinking_messages` | false | Hide reasoning messages |
| `suppress_informational_messages` | false | Hide info/warning messages |

**Safety Settings:**
| Key | Default | Description |
|-----|---------|-------------|
| `yolo_mode` | true | Skip operation confirmations |
| `allow_recursion` | true | Allow recursive agent calls |

**Advanced Settings:**
| Key | Default | Description |
|-----|---------|-------------|
| `disable_mcp` | false | Disable MCP servers entirely |

**OpenAI GPT-5 Specific:**
| Key | Default | Description |
|-----|---------|-------------|
| `openai_reasoning_effort` | "medium" | low/medium/high |
| `openai_verbosity` | "medium" | low/medium/high |

### 3.3 Boolean Value Parsing

Accepted as `true`: "1", "true", "yes", "on" (case-insensitive)
Everything else is `false`

---

## 4. Model Pinning

### 4.1 Purpose

Lock a specific model to an agent, overriding the global default. This allows:
- Different models for different tasks (e.g., code review vs generation)
- Budget control (cheaper models for certain agents)
- Consistency (always use same model for specific agent)

### 4.2 Model Resolution Hierarchy

1. Check agent-specific pinned model
2. If not pinned → use global model from config
3. If no global → use default model (prefers "synthetic-GLM-4.6", else first available)


---

## 5. Model Settings

### 5.1 Per-Model Configuration

Each model can have individual settings. Config keys use sanitized model names (replace `.` `-` `/` with `_`, lowercase).

**Format:** `model_settings_<sanitized_model_name>_<setting>`

### 5.2 Available Settings

| Setting | Type | Range | Applicable Models |
|---------|------|-------|-------------------|
| `temperature` | Numeric | 0.0-1.0 | All |
| `seed` | Integer | 0-999999 | All |
| `top_p` | Numeric | 0.0-1.0 | Most |
| `reasoning_effort` | Choice | low/medium/high | GPT-5 |
| `verbosity` | Choice | low/medium/high | GPT-5 (non-codex) |
| `extended_thinking` | Boolean | true/false | Claude |
| `budget_tokens` | Integer | 1024-131072 | Claude |

### 5.3 Automatic Max Tokens Calculation

When not explicitly set:
```
max_tokens = max(2048, min(context_length * 0.15, 65536))
```

---

## 6. Agent System

### 6.1 Built-in Agents

| Agent Name | Purpose |
|------------|---------|
| `code-agent` | Default full-stack coding assistant |
| `planning-agent` | Task decomposition and coordination |
| `code-reviewer` | General code review (security, performance, design) |


### 6.3 Agent Switching

**Behavior:**
- Preserve message history per agent-sesstion (can switch back and continue)
- Each session tracks its own current agent
- Agent selection persists per session

### 6.4 Sub-Agent Invocation

Agents can invoke other agents for specialized tasks.

**Session ID Patterns:**
- **New session:** Provide base name (e.g., "review-auth") → auto-appended with hash suffix (e.g., "review-auth-a3f2b1")
- **Continue session:** Use full session_id from previous response to maintain conversation
- **One-off:** Leave empty for auto-generated ID

**Features:**
- Session history persisted between invocations
- Metadata tracked: initial_prompt, created_at, message_count, last_updated
- Respects agent-specific model pinning

### 6.5 Agent Discovery

- Built-in agents: Discovered from application package

---

## 7. Tool System

### 7.1 Available Tools

**Agent Coordination:**
| Tool | Description |
|------|-------------|
| `list_agents` | List all available sub-agents |
| `invoke_agent` | Invoke sub-agent with session management |

**File Operations (Read):**
| Tool | Description |
|------|-------------|
| `list_files` | Directory exploration with recursive support |
| `read_file` | Read file contents with token estimation |
| `grep` | Text search across files (up to 200 matches) |

**File Modifications (Write):**
| Tool | Description |
|------|-------------|
| `edit_file` | Create/modify/replace/delete file content |
| `delete_file` | Remove files permanently |

**Shell:**
| Tool | Description |
|------|-------------|
| `agent_run_shell_command` | Execute shell commands with output capture |

**Communication:**
| Tool | Description |
|------|-------------|
| `agent_share_your_reasoning` | Share thought process with user |


### 7.3 File Operations Details

**list_files:**
- Ignore common directories (node_modules, .git, __pycache__, etc. - 100+ patterns)
- Return: path, type, size, depth
- Smart home directory detection limits recursion

**read_file:**
- Return content with line count and token estimate
- Support offset/limit for partial reads
- Handle encoding issues gracefully

**grep:**
- Support regex patterns
- Return: file_path, line_number, line_content
- Respect ignore patterns
- Maximum 200 matches

**edit_file (Swiss-army tool):**
- Modes: write entire content, replace snippets, delete snippets
- Generate diffs for preview
- Trigger permission callbacks

### 7.4 Shell Command Execution

**Features:**
- Output capture (stdout and stderr)
- Timeout handling (default 60s inactivity timeout)
- Process interruption support
- Output truncation (last 256 lines to prevent token overflow)
- Line length limit (256 chars)
- Cross-platform (Windows and POSIX)

**Return Value:**
```
success: boolean (exit code == 0)
command: executed command string
stdout: last 256 lines of stdout
stderr: last 256 lines of stderr
exit_code: process exit code
execution_time: seconds
timeout: was terminated by timeout
user_interrupted: was killed by user
```

---

## 8. MCP Server Integration

### 8.1 Purpose

Model Context Protocol servers provide additional tools to agents. They run as separate processes and communicate via defined protocols.

### 8.2 Server Types

| Type | Communication | Use Case |
|------|--------------|----------|
| SSE | HTTP with Server-Sent Events | Web-based servers |
| Stdio | stdin/stdout process | CLI tools |
| HTTP | Streamable HTTP | REST-style servers |

### 8.3 Server Configuration

**SSE Server:**
```json
{
  "name": "my-server",
  "type": "sse",
  "enabled": true,
  "config": {
    "url": "http://localhost:8000",
    "timeout": 30,
    "read_timeout": 10,
    "headers": {"Authorization": "Bearer token"}
  }
}
```

**Stdio Server:**
```json
{
  "name": "filesystem",
  "type": "stdio",
  "enabled": true,
  "config": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem"],
    "env": {"API_KEY": "value"},
    "cwd": "/path/to/workdir",
    "timeout": 30
  }
}
```

**Validation Rules:**
- Server name: alphanumeric + hyphens/underscores
- Server type: must be "sse", "stdio", or "http"
- URLs: must start with http:// or https://
- Timeouts: must be positive numbers
- Command: required for stdio servers

### 8.4 Server Lifecycle

**States:**
- STOPPED: Disabled/not running
- STARTING: Transitioning to running
- RUNNING: Active and available
- STOPPING: Transitioning to stopped
- ERROR: Failed to initialize
- QUARANTINED: Temporarily disabled due to errors

**Features:**
- Health monitoring (default 30s intervals)
- Automatic recovery after failures
- Error isolation with quarantine
- Circuit breaker for cascading failure prevention
- Consecutive failure tracking
- Event audit trail

---

## 10. Session & Context Management

### 10.1 Conversation History

**What must be stored:**
- Complete message history (system, user, assistant, tool calls)
- Session metadata (name, timestamp, message count, total tokens)
- Auto-saved flag

**Session Naming:**
- Format: `auto_session_YYYYMMDD_HHMMSS`
- Unique per application instance
- Rotate on context load or agent switch

### 10.2 Autosave

**Behavior:**
- Save after each agent response (when enabled)
- Maintain rolling history of recent sessions
- Clean up oldest sessions beyond limit

**Configuration:**
- `auto_save_session`: Enable/disable (default: true)
- `max_saved_sessions`: How many to keep (default: 20)

**Features:**
- Browse and load saved sessions
- Show: timestamps, message counts, token counts
- Preview of recent messages before loading

### 10.3 Context Compaction

**Purpose:** Prevent context overflow by reducing message history size.

**Trigger:** When `(context_used / model_context_length) > compaction_threshold` (default: 85%)

**Strategies:**

1. **Truncation (default):**
   - Simple discard of oldest messages
   - Keep system message + recent messages within protected token budget
   - Fast, no API cost

2. **Summarization:**
   - AI-powered compression
   - Preserve semantic meaning of older messages
   - Use dedicated summarization agent
   - Higher quality but has API cost

**Protected Messages:**
- Recent messages totaling `protected_token_count` tokens (default: 50,000)
- System message always preserved
- Protected tokens capped at 75% of model context length

### 10.4 Context Snapshots

**Save context:**
- Save current message history as named snapshot
- Store messages and metadata

**Load context:**
- Replace current history with saved snapshot
- Rotate autosave ID to prevent overwrite
- Update token count display

### 10.5 Token Estimation

**Message tokens:** Character-based estimation (faster than tokenizer)
**Context overhead:** System prompt + tool definitions + MCP tool definitions

**Display:** Real-time context usage shown during agent execution

---

## 11. System Prompts & Personas

### 11.1 Two-Tier Prompt System

1. **System Prompt:** Core agent behavior instructions
2. **User Prompt:** Optional custom greeting/initial message

### 11.2 Prompt Composition

Final prompt is composed from multiple sources:
```
System Prompt (from agent)
+ Rules (from AGENTS.md files)
+ Callback Injections (from plugins)
```

### 11.3 Rule Files

**Purpose:** Markdown files that augment agent behavior.

**Files checked (in order):**
- Global: `AGENTS.md` or `AGENT.md` (in application config)
- Project: `AGENTS.md` or `AGENT.md` (in current working directory)

**Behavior:** Content is appended to system prompt, allowing per-project customization without modifying agent code.

### 11.5 Claude-Code Prompt Rewriting (CRITICAL)

**When model name starts with `"claude-code"`:**

1. Original system prompt is extracted
2. System prompt is **prepended to first user message**
3. Agent instructions replaced with fixed string:
   ```
   "You are Claude Code, Anthropic's official CLI for Claude."
   ```

**Why:** The claude-code API endpoint does not support custom system prompts. This workaround embeds the agent's personality and instructions in the conversation.

**Applies to:**
- Main agent execution
- Sub-agent invocation
- Summarization agent
- Any component that creates agents

