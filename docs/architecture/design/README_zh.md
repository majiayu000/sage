# Sage Agent Architecture Design Documents

> A comprehensive set of design documents for building a concurrent, asynchronous Code Agent system

## Document Overview

This directory contains the architectural design documents for Sage Agent. These documents follow a systematic approach to software architecture, progressing from high-level vision to detailed implementation specifications.

## Document Index

| # | Document | Description | Priority |
|---|----------|-------------|----------|
| 00 | [Architect Methodology](./00-architect-methodology.md) | Generic methodology for software architects | Reference |
| 01 | [Vision & Constraints](./01-vision-and-constraints.md) | Project scope, goals, and boundaries | P0 |
| 02 | [Domain Model](./02-domain-model.md) | Core entities, relationships, aggregates | P0 |
| 03 | [Concurrency Model](./03-concurrency-model.md) | Async runtime, channels, cancellation | P0 (Critical) |
| 04 | [C4 Architecture](./04-architecture-c4.md) | System structure using C4 model | P1 |
| 05 | [Core Interfaces](./05-core-interfaces.md) | Trait definitions and API contracts | P1 |
| 06 | [State Machines](./06-state-machines.md) | Formal state definitions and transitions | P1 |
| 07 | [Data Flow](./07-data-flow.md) | How data moves through the system | P2 |

## Reading Order

### For New Team Members
1. Start with **00-architect-methodology.md** to understand the approach
2. Read **01-vision-and-constraints.md** to understand what we're building
3. Review **02-domain-model.md** to learn the vocabulary
4. Skim **04-architecture-c4.md** for system overview

### For Implementers
1. **03-concurrency-model.md** - Critical for async/concurrent code
2. **05-core-interfaces.md** - Traits you'll implement
3. **06-state-machines.md** - Behavioral specifications
4. **07-data-flow.md** - How data moves through the system

### For Architects/Tech Leads
1. All documents in numerical order
2. Focus on trade-offs and decision rationale

## Key Design Decisions

| Decision | Choice | Document |
|----------|--------|----------|
| Async Runtime | Tokio | 03-concurrency-model.md |
| Inter-task Communication | Channels (message passing) | 03-concurrency-model.md |
| Agent Architecture | Specialized agent types | 02-domain-model.md |
| Tool Execution | Semaphore-based concurrency control | 03-concurrency-model.md |
| Event Distribution | Broadcast channel | 03-concurrency-model.md |

## Document Status

| Document | Status | Last Updated |
|----------|--------|--------------|
| 00-architect-methodology | Complete | 2024-01 |
| 01-vision-and-constraints | Complete | 2024-01 |
| 02-domain-model | Complete | 2024-01 |
| 03-concurrency-model | Complete | 2024-01 |
| 04-architecture-c4 | Complete | 2024-01 |
| 05-core-interfaces | Complete | 2024-01 |
| 06-state-machines | Complete | 2024-01 |
| 07-data-flow | Complete | 2024-01 |

## Diagram Notation

All diagrams in these documents use ASCII art for maximum portability. The notation:

```
+------------------+     Boxes represent components
|    Component     |
+------------------+

       │
       │ arrows      Lines with arrows show direction
       v

─────────────────    Horizontal lines connect components

┌─────────────────┐
│  Rounded boxes  │  Containers/groups
└─────────────────┘

═══════════════════  Double lines for emphasis
```

## Contributing

When updating these documents:

1. Keep diagrams in ASCII art format
2. Update the "Last Updated" date
3. Ensure consistency with other documents
4. Add entries to the glossary in 02-domain-model.md if new terms are introduced

## Related Documents

- [ADRs (Architecture Decision Records)](../planning/adr/) - Individual decision records
- [Development Planning](../planning/) - Implementation roadmap
- [API Documentation](../api/) - Generated API docs
