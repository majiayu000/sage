# System Overview

## What is Sage Agent?

Sage Agent is a Rust-based LLM agent system designed for software engineering tasks. It provides:

- Multi-provider LLM support (OpenAI, Anthropic, Google)
- Rich tool ecosystem for code manipulation
- Concurrent async execution
- MCP (Model Context Protocol) integration
- Comprehensive error recovery

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Sage Agent System                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   CLI       │  │    SDK      │  │     MCP Servers         │  │
│  │  (sage-cli) │  │  (sage-sdk) │  │  (External Tools)       │  │
│  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘  │
│         │                │                      │                │
│         └────────────────┼──────────────────────┘                │
│                          │                                       │
│  ┌───────────────────────▼───────────────────────────────────┐  │
│  │                     sage-core                              │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐    │  │
│  │  │   Agent     │  │    LLM      │  │     Tools       │    │  │
│  │  │   Engine    │──│   Client    │──│    Executor     │    │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘    │  │
│  │         │                │                  │              │  │
│  │  ┌──────▼──────┐  ┌──────▼──────┐  ┌───────▼───────┐     │  │
│  │  │  Lifecycle  │  │   Events    │  │  Permissions  │     │  │
│  │  │   Manager   │  │    Bus      │  │    System     │     │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘     │  │
│  │         │                │                  │              │  │
│  │  ┌──────▼──────┐  ┌──────▼──────┐  ┌───────▼───────┐     │  │
│  │  │ Cancellation│  │   MCP       │  │    Error      │     │  │
│  │  │  Hierarchy  │  │  Registry   │  │   Recovery    │     │  │
│  │  └─────────────┘  └─────────────┘  └───────────────┘     │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                     sage-tools                             │  │
│  │  [bash] [edit] [json_edit] [codebase_retrieval] [tasks]   │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Request Flow

```
User Request
     │
     ▼
┌─────────────┐
│   Agent     │
│  execute_   │
│    task()   │
└──────┬──────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐
│  Lifecycle  │────▶│   Hooks     │
│   on_task_  │     │  (logging,  │
│   start()   │     │   metrics)  │
└──────┬──────┘     └─────────────┘
       │
       ▼
┌─────────────┐
│   Build     │
│  Messages   │
└──────┬──────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐
│    LLM      │────▶│  Streaming  │
│   Client    │     │   Response  │
│   chat()    │     │    (SSE)    │
└──────┬──────┘     └─────────────┘
       │
       ▼
┌─────────────┐
│   Parse     │
│  Response   │
│ + Tool Calls│
└──────┬──────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐
│   Tool      │────▶│  Permission │
│  Executor   │     │   Check     │
└──────┬──────┘     └─────────────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐
│  Execute    │────▶│   Retry/    │
│   Tools     │     │  Recovery   │
│ (parallel)  │     │             │
└──────┬──────┘     └─────────────┘
       │
       ▼
┌─────────────┐
│  Lifecycle  │
│  on_step_   │
│  complete() │
└──────┬──────┘
       │
       ▼
   Next Step
   or Complete
```

## Key Design Decisions

### 1. Async Runtime
- Tokio for all async operations
- Non-blocking I/O throughout
- Structured concurrency with cancellation tokens

### 2. Multi-Provider LLM
- Unified interface for OpenAI, Anthropic, Google
- Provider-specific optimizations
- Streaming support with SSE decoder

### 3. Tool Execution
- Sequential, batch, and parallel executors
- Permission system with risk levels
- Semaphore-based concurrency control

### 4. Error Handling
- Retry policies with backoff strategies
- Circuit breaker pattern
- Supervisor-based recovery

### 5. Extensibility
- Trait-based tool interface
- MCP protocol for external tools
- Lifecycle hooks for customization
