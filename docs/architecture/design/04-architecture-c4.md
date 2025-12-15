# C4 Architecture Model

> System architecture using the C4 model (Context, Containers, Components, Code)

## 1. Level 1: System Context Diagram

```
+=====================================================================+
|                        SYSTEM CONTEXT                                |
+=====================================================================+
|                                                                      |
|                                                                      |
|                          +-----------+                               |
|                          |           |                               |
|                          | Developer |                               |
|                          |  (User)   |                               |
|                          |           |                               |
|                          +-----+-----+                               |
|                                |                                     |
|                                | Uses CLI/SDK                        |
|                                |                                     |
|                                v                                     |
|                    +-----------------------+                         |
|                    |                       |                         |
|                    |     SAGE AGENT        |                         |
|                    |                       |                         |
|                    |   Concurrent Code     |                         |
|                    |   Agent System        |                         |
|                    |                       |                         |
|                    +-----------+-----------+                         |
|                                |                                     |
|         +----------------------+----------------------+              |
|         |                      |                      |              |
|         v                      v                      v              |
|  +-------------+       +---------------+      +-------------+        |
|  |             |       |               |      |             |        |
|  |  LLM APIs   |       |  File System  |      |    MCP      |        |
|  |             |       |   & Shell     |      |  Servers    |        |
|  | - Anthropic |       |               |      |             |        |
|  | - OpenAI    |       | - Read/Write  |      | - External  |        |
|  | - Google    |       | - Bash exec   |      |   Tools     |        |
|  | - Azure     |       | - Git         |      | - Resources |        |
|  |             |       |               |      |             |        |
|  +-------------+       +---------------+      +-------------+        |
|                                                                      |
+======================================================================+

LEGEND:
+-------+
| User  |  = Person
+-------+

+-------+
|System |  = Software System
+-------+

Arrows = Relationship/Data Flow
```

### Key Relationships

| From | To | Relationship |
|------|-----|--------------|
| Developer | Sage Agent | Uses to complete coding tasks |
| Sage Agent | LLM APIs | Sends prompts, receives responses |
| Sage Agent | File System | Reads/writes files, executes commands |
| Sage Agent | MCP Servers | Extends capabilities via protocol |

---

## 2. Level 2: Container Diagram

```
+=====================================================================+
|                       CONTAINER DIAGRAM                              |
+=====================================================================+
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                        SAGE AGENT SYSTEM                       |  |
|  |                                                                |  |
|  |  +-------------------+                                         |  |
|  |  |                   |                                         |  |
|  |  |     sage-cli      |  Command Line Interface                 |  |
|  |  |                   |  - Interactive mode                     |  |
|  |  |   [Rust Binary]   |  - One-shot mode                        |  |
|  |  |                   |  - Progress display                     |  |
|  |  +--------+----------+                                         |  |
|  |           |                                                    |  |
|  |           | Uses                                               |  |
|  |           v                                                    |  |
|  |  +-------------------+      +-------------------+              |  |
|  |  |                   |      |                   |              |  |
|  |  |     sage-sdk      |      |   sage-server     |              |  |
|  |  |                   |      |   (Future)        |              |  |
|  |  |   [Rust Library]  |      |   [Rust Binary]   |              |  |
|  |  |                   |      |                   |              |  |
|  |  | - Builder API     |      | - HTTP/gRPC API   |              |  |
|  |  | - Session mgmt    |      | - Multi-tenant    |              |  |
|  |  | - Event handling  |      | - Auth            |              |  |
|  |  |                   |      |                   |              |  |
|  |  +--------+----------+      +-------------------+              |  |
|  |           |                                                    |  |
|  |           | Depends on                                         |  |
|  |           v                                                    |  |
|  |  +--------------------------------------------------------+   |  |
|  |  |                                                        |   |  |
|  |  |                      sage-core                         |   |  |
|  |  |                                                        |   |  |
|  |  |   [Rust Library] - Core Agent Engine                   |   |  |
|  |  |                                                        |   |  |
|  |  |   +------------+  +------------+  +------------+       |   |  |
|  |  |   |   Agent    |  |    LLM     |  |   Tool     |       |   |  |
|  |  |   |   Engine   |  |   Client   |  |  Executor  |       |   |  |
|  |  |   +------------+  +------------+  +------------+       |   |  |
|  |  |                                                        |   |  |
|  |  |   +------------+  +------------+  +------------+       |   |  |
|  |  |   |  Message   |  |    MCP     |  |  Sandbox   |       |   |  |
|  |  |   |  Stream    |  |   Client   |  |  Manager   |       |   |  |
|  |  |   +------------+  +------------+  +------------+       |   |  |
|  |  |                                                        |   |  |
|  |  +-------------------------+------------------------------+   |  |
|  |                            |                                  |  |
|  |                            | Uses                             |  |
|  |                            v                                  |  |
|  |  +--------------------------------------------------------+   |  |
|  |  |                                                        |   |  |
|  |  |                     sage-tools                         |   |  |
|  |  |                                                        |   |  |
|  |  |   [Rust Library] - Built-in Tools                      |   |  |
|  |  |                                                        |   |  |
|  |  |   +------+ +------+ +------+ +------+ +------+        |   |  |
|  |  |   | Bash | | Edit | | Read | | Grep | | Glob |        |   |  |
|  |  |   +------+ +------+ +------+ +------+ +------+        |   |  |
|  |  |                                                        |   |  |
|  |  |   +------+ +------+ +------+ +------+ +------+        |   |  |
|  |  |   | Web  | | Web  | | Git  | | Todo | | Task |        |   |  |
|  |  |   |Fetch | |Search| |      | |      | |      |        |   |  |
|  |  |   +------+ +------+ +------+ +------+ +------+        |   |  |
|  |  |                                                        |   |  |
|  |  +--------------------------------------------------------+   |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
+======================================================================+
```

### Container Responsibilities

| Container | Technology | Responsibility |
|-----------|------------|----------------|
| sage-cli | Rust binary | User interaction, terminal UI |
| sage-sdk | Rust library | Programmatic API for embedders |
| sage-core | Rust library | Core agent logic, LLM integration |
| sage-tools | Rust library | Built-in tool implementations |
| sage-server | Rust binary | Future HTTP/gRPC server |

---

## 3. Level 3: Component Diagram (sage-core)

```
+=====================================================================+
|                    SAGE-CORE COMPONENT DIAGRAM                       |
+=====================================================================+
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                         PUBLIC API                             |  |
|  |                                                                |  |
|  |   SageBuilder    Session    AgentHandle    EventSubscriber    |  |
|  |                                                                |  |
|  +--------------------------------+------------------------------+  |
|                                   |                                  |
|  +--------------------------------v------------------------------+  |
|  |                        ORCHESTRATION LAYER                    |  |
|  |                                                                |  |
|  |  +------------------+    +------------------+                 |  |
|  |  |                  |    |                  |                 |  |
|  |  |     Session      |    |      Agent       |                 |  |
|  |  |   Orchestrator   |    |     Factory      |                 |  |
|  |  |                  |    |                  |                 |  |
|  |  | - Session state  |    | - Agent creation |                 |  |
|  |  | - Agent mgmt     |    | - Tool binding   |                 |  |
|  |  | - Event routing  |    | - Config apply   |                 |  |
|  |  |                  |    |                  |                 |  |
|  |  +--------+---------+    +--------+---------+                 |  |
|  |           |                       |                           |  |
|  +-----------+-----------------------+---------------------------+  |
|              |                       |                              |
|  +-----------v-----------------------v---------------------------+  |
|  |                         AGENT LAYER                           |  |
|  |                                                                |  |
|  |  +----------------+  +----------------+  +----------------+   |  |
|  |  |                |  |                |  |                |   |  |
|  |  |  Base Agent    |  |  Explore Agent |  |   Plan Agent   |   |  |
|  |  |  (General)     |  |    (Fast)      |  |  (Architect)   |   |  |
|  |  |                |  |                |  |                |   |  |
|  |  +-------+--------+  +-------+--------+  +-------+--------+   |  |
|  |          |                   |                   |            |  |
|  |          +-------------------+-------------------+            |  |
|  |                              |                                |  |
|  +------------------------------+--------------------------------+  |
|                                 |                                   |
|  +------------------------------v--------------------------------+  |
|  |                        EXECUTION LAYER                        |  |
|  |                                                                |  |
|  |  +------------------+    +------------------+                 |  |
|  |  |                  |    |                  |                 |  |
|  |  |  Task Executor   |    |  Tool Executor   |                 |  |
|  |  |                  |    |                  |                 |  |
|  |  | - Step execution |    | - Concurrency    |                 |  |
|  |  | - State machine  |    | - Permissions    |                 |  |
|  |  | - Trajectory     |    | - Sandboxing     |                 |  |
|  |  |                  |    |                  |                 |  |
|  |  +--------+---------+    +--------+---------+                 |  |
|  |           |                       |                           |  |
|  +-----------+-----------------------+---------------------------+  |
|              |                       |                              |
|  +-----------v-----------------------v---------------------------+  |
|  |                       STREAMING LAYER                         |  |
|  |                                                                |  |
|  |  +------------------+    +------------------+                 |  |
|  |  |                  |    |                  |                 |  |
|  |  | MessageStream    |    |    Event Bus    |                 |  |
|  |  |   Handler        |    |                  |                 |  |
|  |  |                  |    | - Broadcast      |                 |  |
|  |  | - SSE parsing    |    | - Subscribers    |                 |  |
|  |  | - Delta assembly |    | - Backpressure   |                 |  |
|  |  |                  |    |                  |                 |  |
|  |  +--------+---------+    +--------+---------+                 |  |
|  |           |                       |                           |  |
|  +-----------+-----------------------+---------------------------+  |
|              |                       |                              |
|  +-----------v-----------------------v---------------------------+  |
|  |                     INFRASTRUCTURE LAYER                      |  |
|  |                                                                |  |
|  |  +----------+  +----------+  +----------+  +----------+      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  |   LLM    |  |   MCP    |  | Sandbox  |  |  Cache   |      |  |
|  |  |  Client  |  |  Client  |  | Manager  |  | Manager  |      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  +----------+  +----------+  +----------+  +----------+      |  |
|  |                                                                |  |
|  |  +----------+  +----------+  +----------+                     |  |
|  |  |          |  |          |  |          |                     |  |
|  |  |  Config  |  |Telemetry |  |Trajectory|                     |  |
|  |  |  Loader  |  | Reporter |  | Recorder |                     |  |
|  |  |          |  |          |  |          |                     |  |
|  |  +----------+  +----------+  +----------+                     |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
+======================================================================+
```

### Component Descriptions

| Component | Layer | Responsibility |
|-----------|-------|----------------|
| Session Orchestrator | Orchestration | Manages session lifecycle, routes events |
| Agent Factory | Orchestration | Creates configured agent instances |
| Base Agent | Agent | General-purpose task execution |
| Explore Agent | Agent | Fast codebase exploration |
| Plan Agent | Agent | Architecture and planning |
| Task Executor | Execution | Runs agent tasks, manages state |
| Tool Executor | Execution | Executes tools with concurrency control |
| MessageStream Handler | Streaming | Parses SSE, assembles messages |
| Event Bus | Streaming | Distributes events to subscribers |
| LLM Client | Infrastructure | Communicates with LLM APIs |
| MCP Client | Infrastructure | Implements MCP protocol |
| Sandbox Manager | Infrastructure | Provides execution isolation |
| Cache Manager | Infrastructure | Caches LLM responses |

---

## 4. Level 3: Component Diagram (sage-tools)

```
+=====================================================================+
|                   SAGE-TOOLS COMPONENT DIAGRAM                       |
+=====================================================================+
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                      TOOL REGISTRY                             |  |
|  |                                                                |  |
|  |   register()    get()    list()    get_schemas()              |  |
|  |                                                                |  |
|  +--------------------------------+------------------------------+  |
|                                   |                                  |
|  +--------------------------------v------------------------------+  |
|  |                     FILE SYSTEM TOOLS                         |  |
|  |                                                                |  |
|  |  +----------+  +----------+  +----------+  +----------+      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  |   Read   |  |   Edit   |  |  Write   |  |  Glob    |      |  |
|  |  |   Tool   |  |   Tool   |  |   Tool   |  |   Tool   |      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  | Read file|  | Surgical |  | Create/  |  | Pattern  |      |  |
|  |  | contents |  | edits    |  | overwrite|  | matching |      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  +----------+  +----------+  +----------+  +----------+      |  |
|  |                                                                |  |
|  |  +----------+                                                 |  |
|  |  |          |                                                 |  |
|  |  |   Grep   |                                                 |  |
|  |  |   Tool   |                                                 |  |
|  |  |          |                                                 |  |
|  |  | Content  |                                                 |  |
|  |  | search   |                                                 |  |
|  |  |          |                                                 |  |
|  |  +----------+                                                 |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                     EXECUTION TOOLS                            |  |
|  |                                                                |  |
|  |  +----------+  +----------+  +----------+                     |  |
|  |  |          |  |          |  |          |                     |  |
|  |  |   Bash   |  |   Git    |  |  Docker  |                     |  |
|  |  |   Tool   |  |   Tool   |  |   Tool   |                     |  |
|  |  |          |  |          |  |  (Future)|                     |  |
|  |  | Shell    |  | Version  |  |          |                     |  |
|  |  | commands |  | control  |  | Container|                     |  |
|  |  |          |  |          |  | ops      |                     |  |
|  |  +----------+  +----------+  +----------+                     |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                      NETWORK TOOLS                             |  |
|  |                                                                |  |
|  |  +----------+  +----------+  +----------+                     |  |
|  |  |          |  |          |  |          |                     |  |
|  |  | WebFetch |  |WebSearch |  | Browser  |                     |  |
|  |  |   Tool   |  |   Tool   |  |   Tool   |                     |  |
|  |  |          |  |          |  |          |                     |  |
|  |  | HTTP GET |  | Web      |  | Browser  |                     |  |
|  |  | + parse  |  | search   |  | automate |                     |  |
|  |  |          |  |          |  |          |                     |  |
|  |  +----------+  +----------+  +----------+                     |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                    MANAGEMENT TOOLS                            |  |
|  |                                                                |  |
|  |  +----------+  +----------+  +----------+  +----------+      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  |  Todo    |  |  Task    |  | AskUser  |  |  Memory  |      |  |
|  |  |  Write   |  |  Spawn   |  |   Tool   |  |   Tool   |      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  | Task     |  | Sub-agent|  | User     |  | Context  |      |  |
|  |  | tracking |  | spawning |  | queries  |  | storage  |      |  |
|  |  |          |  |          |  |          |  |          |      |  |
|  |  +----------+  +----------+  +----------+  +----------+      |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
+======================================================================+
```

---

## 5. Component Interactions

### 5.1 Chat Flow Sequence

```
+------------------------------------------------------------------+
|                      CHAT FLOW SEQUENCE                           |
+------------------------------------------------------------------+
|                                                                   |
|  User        CLI       Session      Agent      LLM       Tool    |
|   |           |           |           |         |          |      |
|   |  input    |           |           |         |          |      |
|   |---------->|           |           |         |          |      |
|   |           |  chat()   |           |         |          |      |
|   |           |---------->|           |         |          |      |
|   |           |           | dispatch  |         |          |      |
|   |           |           |---------->|         |          |      |
|   |           |           |           | stream  |          |      |
|   |           |           |           |-------->|          |      |
|   |           |           |           |         |          |      |
|   |           |           |           |<--------|          |      |
|   |           |           |           | events  |          |      |
|   |           |           |           |         |          |      |
|   |           |           |           | tool_use|          |      |
|   |           |           |           |------------------>|      |
|   |           |           |           |         |          |      |
|   |           |           |           |<------------------|      |
|   |           |           |           | result  |          |      |
|   |           |           |           |         |          |      |
|   |           |           |           | continue|          |      |
|   |           |           |           |-------->|          |      |
|   |           |           |           |         |          |      |
|   |           |           |<----------|         |          |      |
|   |           |           | complete  |         |          |      |
|   |           |<----------|           |         |          |      |
|   |           | response  |           |         |          |      |
|   |<----------|           |           |         |          |      |
|   |  output   |           |           |         |          |      |
|   |           |           |           |         |          |      |
|                                                                   |
+------------------------------------------------------------------+
```

### 5.2 Event Flow

```
+------------------------------------------------------------------+
|                        EVENT FLOW                                 |
+------------------------------------------------------------------+
|                                                                   |
|  +-----------+                                                    |
|  |    LLM    |                                                    |
|  |  Response |                                                    |
|  +-----+-----+                                                    |
|        |                                                          |
|        | SSE events                                               |
|        v                                                          |
|  +-----+-----+                                                    |
|  | Message   |                                                    |
|  | Stream    |                                                    |
|  | Handler   |                                                    |
|  +-----+-----+                                                    |
|        |                                                          |
|        | StreamEvents                                             |
|        v                                                          |
|  +-----+-----+                                                    |
|  |   Event   |------------------------------------------+         |
|  |    Bus    |----------------------------+             |         |
|  |           |--------------+             |             |         |
|  +-----------+              |             |             |         |
|        |                    |             |             |         |
|        v                    v             v             v         |
|  +-----------+      +-----------+  +-----------+  +-----------+  |
|  |    UI     |      |   Agent   |  |Trajectory |  | Telemetry |  |
|  |  Renderer |      |  Handler  |  |  Recorder |  |  Reporter |  |
|  +-----------+      +-----------+  +-----------+  +-----------+  |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 6. Deployment View

```
+=====================================================================+
|                       DEPLOYMENT VIEW                                |
+=====================================================================+
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                    DEVELOPER MACHINE                           |  |
|  |                                                                |  |
|  |  +-----------------------+                                    |  |
|  |  |                       |                                    |  |
|  |  |    Terminal / IDE     |                                    |  |
|  |  |                       |                                    |  |
|  |  +-----------+-----------+                                    |  |
|  |              |                                                 |  |
|  |              | invokes                                         |  |
|  |              v                                                 |  |
|  |  +-----------------------+      +------------------------+    |  |
|  |  |                       |      |                        |    |  |
|  |  |     sage (binary)     |<---->|    MCP Servers         |    |  |
|  |  |                       |      |    (local processes)   |    |  |
|  |  |  - sage-cli           |      |                        |    |  |
|  |  |  - sage-core          |      +------------------------+    |  |
|  |  |  - sage-tools         |                                    |  |
|  |  |                       |                                    |  |
|  |  +-----------+-----------+                                    |  |
|  |              |                                                 |  |
|  |              | HTTPS                                           |  |
|  +--------------|------------------------------------------------+  |
|                 |                                                    |
|                 v                                                    |
|  +---------------------------------------------------------------+  |
|  |                       CLOUD SERVICES                           |  |
|  |                                                                |  |
|  |  +------------+  +------------+  +------------+               |  |
|  |  |            |  |            |  |            |               |  |
|  |  | Anthropic  |  |   OpenAI   |  |   Google   |               |  |
|  |  |    API     |  |    API     |  |  Vertex    |               |  |
|  |  |            |  |            |  |            |               |  |
|  |  +------------+  +------------+  +------------+               |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
+======================================================================+
```

---

## 7. Technology Mapping

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Binary** | Rust + Clap | CLI parsing, binary output |
| **Async Runtime** | Tokio | Async I/O, task scheduling |
| **HTTP** | Reqwest | HTTP client for API calls |
| **Serialization** | Serde + JSON | Data serialization |
| **Streaming** | futures + async-stream | Stream processing |
| **Channels** | tokio::sync | Inter-task communication |
| **Terminal UI** | crossterm + ratatui | Terminal rendering |
| **Configuration** | config-rs | Configuration loading |
| **Logging** | tracing | Structured logging |
| **Error Handling** | thiserror + anyhow | Error types |

---

## 8. Cross-Cutting Concerns

```
+------------------------------------------------------------------+
|                   CROSS-CUTTING CONCERNS                          |
+------------------------------------------------------------------+
|                                                                   |
|  +------------------------------------------------------------+  |
|  |                        SECURITY                             |  |
|  |                                                             |  |
|  |  - Permission checks before tool execution                  |  |
|  |  - Input validation against schemas                         |  |
|  |  - Path traversal prevention                                |  |
|  |  - Command injection prevention                             |  |
|  |  - API key encryption in memory                             |  |
|  +------------------------------------------------------------+  |
|                                                                   |
|  +------------------------------------------------------------+  |
|  |                       OBSERVABILITY                         |  |
|  |                                                             |  |
|  |  - Structured logging (tracing)                             |  |
|  |  - Metrics collection (prometheus)                          |  |
|  |  - Distributed tracing (OpenTelemetry)                      |  |
|  |  - Trajectory recording for replay                          |  |
|  +------------------------------------------------------------+  |
|                                                                   |
|  +------------------------------------------------------------+  |
|  |                    ERROR HANDLING                           |  |
|  |                                                             |  |
|  |  - Typed errors per layer                                   |  |
|  |  - Error propagation with context                           |  |
|  |  - Graceful degradation                                     |  |
|  |  - User-friendly error messages                             |  |
|  +------------------------------------------------------------+  |
|                                                                   |
|  +------------------------------------------------------------+  |
|  |                    CONFIGURATION                            |  |
|  |                                                             |  |
|  |  - File-based config (TOML/YAML)                            |  |
|  |  - Environment variables                                    |  |
|  |  - CLI argument overrides                                   |  |
|  |  - Runtime reconfiguration                                  |  |
|  +------------------------------------------------------------+  |
|                                                                   |
+------------------------------------------------------------------+
```
