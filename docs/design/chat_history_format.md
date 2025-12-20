# Sage Agent Chat History JSON Format Design

## Overview

This document describes the enhanced chat history format for Sage Agent, inspired by Claude Code's design. The format supports:

1. **Message chaining** with `parentUuid` for conversation branching
2. **Session context** embedded in each message
3. **File history snapshots** for undo/redo capability
4. **Todo list snapshots** to track task state per message
5. **Thinking metadata** for extended thinking control

## Message Types

### 1. User Message

```json
{
  "type": "user",
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "parentUuid": null,
  "timestamp": "2025-12-21T01:30:00.000Z",
  "sessionId": "session-123",
  "version": "0.1.0",

  "context": {
    "cwd": "/Users/lifcc/Desktop/code/AI/agent/sage",
    "gitBranch": "main",
    "platform": "macos",
    "userType": "external"
  },

  "message": {
    "role": "user",
    "content": "Create a weather website"
  },

  "thinkingMetadata": {
    "level": "high",
    "disabled": false,
    "triggers": []
  },

  "todos": [],

  "isSidechain": false
}
```

### 2. Assistant Message

```json
{
  "type": "assistant",
  "uuid": "550e8400-e29b-41d4-a716-446655440001",
  "parentUuid": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2025-12-21T01:30:05.000Z",
  "sessionId": "session-123",
  "version": "0.1.0",

  "context": {
    "cwd": "/Users/lifcc/Desktop/code/AI/agent/sage",
    "gitBranch": "main",
    "platform": "macos"
  },

  "message": {
    "role": "assistant",
    "content": "I'll create a weather website for you.",
    "toolCalls": [
      {
        "id": "call_001",
        "name": "bash",
        "arguments": {
          "command": "mkdir -p weather_app"
        }
      }
    ]
  },

  "usage": {
    "inputTokens": 1500,
    "outputTokens": 200,
    "cacheReadTokens": 500,
    "cacheWriteTokens": 0
  },

  "todos": [
    {
      "content": "Create weather app directory",
      "status": "completed",
      "activeForm": "Creating weather app directory"
    },
    {
      "content": "Create HTML file",
      "status": "in_progress",
      "activeForm": "Creating HTML file"
    }
  ],

  "isSidechain": false
}
```

### 3. Tool Result Message

```json
{
  "type": "tool_result",
  "uuid": "550e8400-e29b-41d4-a716-446655440002",
  "parentUuid": "550e8400-e29b-41d4-a716-446655440001",
  "timestamp": "2025-12-21T01:30:06.000Z",
  "sessionId": "session-123",

  "toolResults": [
    {
      "toolCallId": "call_001",
      "toolName": "bash",
      "content": "Directory created successfully",
      "success": true,
      "error": null
    }
  ],

  "isSidechain": false
}
```

### 4. File History Snapshot

```json
{
  "type": "file_history_snapshot",
  "messageId": "550e8400-e29b-41d4-a716-446655440001",
  "timestamp": "2025-12-21T01:30:05.500Z",
  "isSnapshotUpdate": false,

  "snapshot": {
    "trackedFiles": {
      "weather_app/index.html": {
        "originalContent": null,
        "contentHash": "abc123",
        "size": 0,
        "state": "created"
      }
    },
    "fileBackups": {
      "weather_app/style.css": {
        "backupPath": ".sage/backups/style.css.backup",
        "originalHash": "def456"
      }
    }
  }
}
```

## Data Structures (Rust)

### EnhancedMessage

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMessage {
    /// Message type
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// Unique message identifier
    pub uuid: String,

    /// Parent message UUID (for message chains)
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,

    /// Sage Agent version
    pub version: String,

    /// Session context
    pub context: SessionContext,

    /// Message content
    pub message: MessageContent,

    /// Token usage (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,

    /// Thinking metadata
    #[serde(rename = "thinkingMetadata")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_metadata: Option<ThinkingMetadata>,

    /// Todo list snapshot
    #[serde(default)]
    pub todos: Vec<TodoItem>,

    /// Whether this is a sidechain (branch)
    #[serde(rename = "isSidechain")]
    #[serde(default)]
    pub is_sidechain: bool,
}
```

### SessionContext

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    /// Current working directory
    pub cwd: PathBuf,

    /// Current git branch (if in git repo)
    #[serde(rename = "gitBranch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,

    /// Platform (macos, linux, windows)
    pub platform: String,

    /// User type (external, internal)
    #[serde(rename = "userType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
}
```

### ThinkingMetadata

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingMetadata {
    /// Thinking level (none, low, medium, high)
    pub level: ThinkingLevel,

    /// Whether extended thinking is disabled
    pub disabled: bool,

    /// Triggers that activated thinking
    #[serde(default)]
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    None,
    Low,
    Medium,
    High,
}
```

### FileHistorySnapshot

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHistorySnapshot {
    /// Snapshot type
    #[serde(rename = "type")]
    pub snapshot_type: String, // "file_history_snapshot"

    /// Associated message ID
    #[serde(rename = "messageId")]
    pub message_id: String,

    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,

    /// Whether this is an update to existing snapshot
    #[serde(rename = "isSnapshotUpdate")]
    pub is_snapshot_update: bool,

    /// Actual snapshot data
    pub snapshot: FileSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    /// Tracked files with their state
    #[serde(rename = "trackedFiles")]
    pub tracked_files: HashMap<PathBuf, TrackedFileState>,

    /// File backups for undo
    #[serde(rename = "fileBackups")]
    pub file_backups: HashMap<PathBuf, FileBackup>,
}
```

## Storage Format

### Directory Structure

```
.sage/
├── sessions/
│   ├── session-123/
│   │   ├── messages.jsonl        # JSONL format for messages
│   │   ├── snapshots.jsonl       # File history snapshots
│   │   ├── metadata.json         # Session metadata
│   │   └── backups/              # File backups
│   │       ├── file1.backup
│   │       └── file2.backup
│   └── session-456/
│       └── ...
└── config.json
```

### JSONL Format

Each line in `messages.jsonl` is a complete JSON message:

```
{"type":"user","uuid":"...","parentUuid":null,...}
{"type":"assistant","uuid":"...","parentUuid":"...",...}
{"type":"tool_result","uuid":"...","parentUuid":"...",...}
{"type":"file_history_snapshot","messageId":"...",...}
```

## Key Features

### 1. Message Chaining (parentUuid)

Enables:
- Linear conversation tracking
- Conversation branching (sidechains)
- Undo to specific message point
- Message tree visualization

### 2. Session Context Per Message

Each message captures:
- Current working directory
- Git branch state
- Platform info
- User type

This ensures:
- Reproducible context
- Path validation
- Environment awareness

### 3. File History Snapshots

Linked to messages via `messageId`:
- Track file changes per tool execution
- Enable file-level undo
- Store backups for restoration
- Content hashing for verification

### 4. Todo Snapshots

Each message stores current todo state:
- Task progression tracking
- State restoration on undo
- Progress visualization

### 5. Thinking Metadata

Controls extended thinking:
- Adjustable thinking levels
- Trigger-based activation
- Can be disabled per message

## Migration Path

1. Existing `ConversationMessage` continues to work
2. New `EnhancedMessage` adds additional fields
3. Serialization handles both formats
4. Gradual migration during session save

## Comparison with Claude Code

| Feature | Claude Code | Sage Agent (New) |
|---------|-------------|------------------|
| Message UUID | ✅ | ✅ |
| Parent UUID | ✅ | ✅ |
| Session Context | ✅ (cwd, gitBranch) | ✅ (cwd, gitBranch, platform) |
| File Snapshots | ✅ | ✅ |
| Todo Snapshots | ✅ | ✅ |
| Thinking Metadata | ✅ | ✅ |
| Sidechain Support | ✅ | ✅ |
| JSONL Storage | ✅ | ✅ |
| Version Tracking | ✅ | ✅ |

## Implementation Plan

1. **Phase 1**: Add new types to `session/types.rs`
2. **Phase 2**: Update message creation in agent execution
3. **Phase 3**: Implement JSONL storage
4. **Phase 4**: Add file snapshot integration
5. **Phase 5**: Add undo/redo commands
6. **Phase 6**: Add conversation branching UI
