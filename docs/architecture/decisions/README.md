# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) for the Sage Agent project. ADRs document significant architectural decisions, their context, and consequences.

## What is an ADR?

An Architecture Decision Record captures an important architectural decision made along with its context and consequences. Each ADR follows a standard format:

- **Title**: What was decided
- **Status**: Accepted, Proposed, Deprecated, Superseded
- **Context**: What factors influenced the decision
- **Decision**: What we decided to do
- **Consequences**: What happens as a result (positive and negative)

## Index

### Core Architecture

- [ADR-0001: Multi-Crate Workspace Architecture](0001-multi-crate-workspace.md)
  - Decision to structure the project as four separate crates (sage-core, sage-cli, sage-sdk, sage-tools)
  - Rationale: Code reuse, clear boundaries, flexible deployment

- [ADR-0002: Tokio as Async Runtime](0002-async-runtime-tokio.md)
  - Decision to use Tokio for asynchronous operations
  - Rationale: Mature ecosystem, full feature set, streaming support, LLM integration

### System Design

- [ADR-0003: Tool Trait Design](0003-tool-trait-design.md)
  - Decision to use trait-based abstraction for tools with rich metadata
  - Rationale: Extensibility, type safety, permission control, concurrency management

- [ADR-0004: LLM Provider Abstraction](0004-llm-provider-abstraction.md)
  - Decision to use unified trait with provider-specific enum wrapper
  - Rationale: Provider isolation, easy addition of new providers, unified error handling

## Reading Guide

### For New Contributors

Start with these ADRs to understand the fundamental architecture:
1. ADR-0001: Multi-Crate Workspace - Understand the project structure
2. ADR-0002: Tokio Runtime - Understand why everything is async
3. ADR-0003: Tool Trait - Understand how to add new tools

### For Integration Partners

If you're embedding Sage Agent in your application:
- ADR-0001: Learn about the sage-sdk vs sage-core separation
- ADR-0004: Understand how to configure different LLM providers

### For Tool Developers

If you're creating custom tools:
- ADR-0003: Complete guide to the Tool trait and its design

## ADR Lifecycle

### Status Values

- **Proposed**: Under discussion, not yet implemented
- **Accepted**: Decision made and implemented
- **Deprecated**: No longer recommended, but still supported
- **Superseded**: Replaced by a newer decision (reference the new ADR)

### Creating a New ADR

1. Copy the template (create one if it doesn't exist)
2. Number sequentially (next number is 0005)
3. Write in present tense ("we decide" not "we decided")
4. Include code examples where helpful
5. List alternatives considered and why rejected
6. Update this README index

### Updating an ADR

ADRs are **mostly immutable**. If a decision changes:
1. Create a new ADR that supersedes the old one
2. Mark the old ADR as "Superseded by ADR-XXXX"
3. Link the new ADR to the old one for context

Minor corrections (typos, clarifications) can be made directly.

## Format

Each ADR follows this structure:

```markdown
# ADR-XXXX: Title

## Status

Accepted | Proposed | Deprecated | Superseded by ADR-YYYY

## Context

What is the issue we're addressing? What factors influence this decision?

## Decision

What did we decide to do? Be specific and actionable.

## Consequences

What becomes easier or harder as a result of this decision?

### Positive
- List of benefits

### Negative
- List of drawbacks

### Alternative Approaches Considered
- What else did we consider?
- Why did we reject it?
```

## References

- [Michael Nygard's ADR Template](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [ADR GitHub Organization](https://adr.github.io/)
- [When to Write an ADR](https://engineering.atspotify.com/2020/04/when-should-i-write-an-architecture-decision-record/)

## Future ADRs

Potential topics for future ADRs:
- Error handling strategy (SageError design)
- Configuration management (JSON vs TOML vs environment variables)
- Trajectory recording format and storage
- Permission system design
- UI/UX patterns for CLI
- Testing strategy (unit vs integration)
- Release and versioning strategy
