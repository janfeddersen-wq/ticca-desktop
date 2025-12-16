-- Initial database schema for Ticca Desktop
-- Version: 1
-- Description: Creates core tables for conversations, messages, settings, and sessions

-- Conversations table
-- Stores chat sessions with AI agents
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Messages table
-- Stores individual messages within conversations
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool')),
    content TEXT NOT NULL,
    tool_calls TEXT,      -- JSON array of tool calls if role is 'assistant'
    tool_call_id TEXT,    -- If role is 'tool', the ID of the tool call this responds to
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    token_count INTEGER
);

-- Index for efficient message lookups by conversation
CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);

-- Index for message timestamp queries
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);

-- Settings table
-- Key-value store for persistent configuration
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Sessions table (for agent sub-sessions)
-- Stores persistent session state for agents
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    agent_name TEXT NOT NULL,
    initial_prompt TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    message_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT  -- JSON for additional data
);

-- Index for session lookups by agent name
CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent_name);
