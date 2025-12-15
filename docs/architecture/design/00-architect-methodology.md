# Software Architect Methodology

> A systematic approach to software architecture design - independent of any specific project

## Overview

This document outlines a methodology for software architects to design systems effectively.
It is technology-agnostic and can be applied to any software project.

---

## 1. The Architect's Mindset

### 1.1 Core Responsibilities

```
+=====================================================================+
|                    ARCHITECT'S CORE RESPONSIBILITIES                 |
+=====================================================================+
|                                                                      |
|  1. TECHNICAL VISION                                                |
|     - Define the system's technical direction                       |
|     - Ensure alignment with business goals                          |
|     - Balance innovation with pragmatism                            |
|                                                                      |
|  2. STRUCTURAL INTEGRITY                                            |
|     - Design the system's organization                              |
|     - Define component boundaries                                   |
|     - Establish patterns and conventions                            |
|                                                                      |
|  3. QUALITY ATTRIBUTES                                              |
|     - Define and prioritize NFRs                                    |
|     - Design for -ilities (scalability, reliability, etc.)          |
|     - Balance competing quality attributes                          |
|                                                                      |
|  4. DECISION MAKING                                                 |
|     - Make and document architectural decisions                     |
|     - Manage technical debt consciously                             |
|     - Know when NOT to decide (defer until necessary)               |
|                                                                      |
|  5. COMMUNICATION                                                   |
|     - Translate between business and technical                      |
|     - Create artifacts that communicate architecture                |
|     - Mentor and guide development team                             |
|                                                                      |
+======================================================================+
```

### 1.2 The Architect's Paradox

```
"The architect must make decisions early,
 when they know the least,
 that will have the most lasting impact."

 Therefore:

 - Defer decisions until the "last responsible moment"
 - Make decisions reversible where possible
 - Gather information actively before committing
 - Design for change, not just current requirements
```

---

## 2. The Design Process

### 2.1 Phase Overview

```
+------------------------------------------------------------------+
|                     ARCHITECTURE DESIGN PHASES                    |
+------------------------------------------------------------------+
|                                                                   |
|  Phase 1: UNDERSTAND                                              |
|  ─────────────────────                                            |
|  Inputs:  Requirements, constraints, context                      |
|  Outputs: Problem statement, scope definition                     |
|  Time:    10-20% of architecture effort                           |
|                                                                   |
|       │                                                           |
|       v                                                           |
|                                                                   |
|  Phase 2: ENVISION                                                |
|  ─────────────────                                                |
|  Inputs:  Problem statement, quality attributes                   |
|  Outputs: Architecture options, trade-off analysis                |
|  Time:    20-30% of architecture effort                           |
|                                                                   |
|       │                                                           |
|       v                                                           |
|                                                                   |
|  Phase 3: DESIGN                                                  |
|  ───────────────                                                  |
|  Inputs:  Selected approach, constraints                          |
|  Outputs: Detailed design documents                               |
|  Time:    30-40% of architecture effort                           |
|                                                                   |
|       │                                                           |
|       v                                                           |
|                                                                   |
|  Phase 4: VALIDATE                                                |
|  ────────────────                                                 |
|  Inputs:  Design documents, prototypes                            |
|  Outputs: Validated architecture, risk assessment                 |
|  Time:    10-20% of architecture effort                           |
|                                                                   |
|       │                                                           |
|       v                                                           |
|                                                                   |
|  Phase 5: COMMUNICATE                                             |
|  ────────────────────                                             |
|  Inputs:  Validated design                                        |
|  Outputs: Architecture documentation, presentations               |
|  Time:    10-20% of architecture effort                           |
|                                                                   |
+------------------------------------------------------------------+
```

### 2.2 Phase 1: Understand

```
UNDERSTAND: Before solving, ensure you understand the problem

Key Questions:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  WHAT is the system supposed to do?                             │
│  ├── Core functionality                                         │
│  ├── User journeys                                              │
│  └── Integration points                                         │
│                                                                 │
│  WHO will use it?                                               │
│  ├── Primary users                                              │
│  ├── Secondary stakeholders                                     │
│  └── Operations teams                                           │
│                                                                 │
│  WHY is it being built?                                         │
│  ├── Business drivers                                           │
│  ├── Success metrics                                            │
│  └── Non-goals (equally important)                              │
│                                                                 │
│  WHAT are the constraints?                                      │
│  ├── Technical (platforms, languages, integrations)             │
│  ├── Organizational (team skills, timeline, budget)             │
│  ├── Regulatory (compliance, security requirements)             │
│  └── Political (existing systems, stakeholder preferences)      │
│                                                                 │
│  HOW will success be measured?                                  │
│  ├── Functional correctness                                     │
│  ├── Performance targets                                        │
│  └── Quality attribute scenarios                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Techniques:
- Stakeholder interviews
- Requirements workshops
- Domain modeling sessions
- Context diagrams
- User story mapping
```

### 2.3 Phase 2: Envision

```
ENVISION: Generate options before committing to one

Steps:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  1. IDENTIFY ARCHITECTURALLY SIGNIFICANT REQUIREMENTS (ASRs)    │
│     ├── Requirements that shape the architecture                │
│     ├── High-risk requirements                                  │
│     ├── Novel or unfamiliar requirements                        │
│     └── Non-negotiable constraints                              │
│                                                                 │
│  2. GENERATE MULTIPLE OPTIONS                                   │
│     ├── At least 2-3 distinct approaches                        │
│     ├── Include the "obvious" solution                          │
│     ├── Include a "radical" alternative                         │
│     └── Consider hybrid approaches                              │
│                                                                 │
│  3. EVALUATE OPTIONS AGAINST ASRs                               │
│     ├── Create evaluation matrix                                │
│     ├── Score each option                                       │
│     ├── Identify trade-offs                                     │
│     └── Document assumptions                                    │
│                                                                 │
│  4. SELECT AND JUSTIFY                                          │
│     ├── Choose based on weighted criteria                       │
│     ├── Document rationale                                      │
│     ├── Acknowledge trade-offs                                  │
│     └── Define mitigation strategies                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Option Evaluation Matrix:
┌──────────────┬────────┬────────┬────────┐
│ Criterion    │ Opt A  │ Opt B  │ Opt C  │
├──────────────┼────────┼────────┼────────┤
│ Scalability  │ High   │ Medium │ High   │
│ Complexity   │ Medium │ Low    │ High   │
│ Cost         │ Medium │ Low    │ High   │
│ Time to MVP  │ Medium │ Fast   │ Slow   │
│ Team fit     │ Good   │ Great  │ Poor   │
├──────────────┼────────┼────────┼────────┤
│ TOTAL        │ 7/10   │ 8/10   │ 6/10   │
└──────────────┴────────┴────────┴────────┘
```

### 2.4 Phase 3: Design

```
DESIGN: Create detailed technical specifications

Design Artifact Hierarchy:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  Level 1: VISION & CONSTRAINTS                                  │
│  ────────────────────────────                                   │
│  What: High-level goals and boundaries                          │
│  Who reads: All stakeholders                                    │
│  Format: Vision document, constraint matrix                     │
│                                                                 │
│  Level 2: DOMAIN MODEL                                          │
│  ────────────────────────                                       │
│  What: Core entities and relationships                          │
│  Who reads: Developers, domain experts                          │
│  Format: UML class diagrams, entity definitions                 │
│                                                                 │
│  Level 3: STRUCTURAL DESIGN                                     │
│  ────────────────────────────                                   │
│  What: System decomposition                                     │
│  Who reads: Tech leads, senior developers                       │
│  Format: C4 diagrams, component specifications                  │
│                                                                 │
│  Level 4: BEHAVIORAL DESIGN                                     │
│  ────────────────────────────                                   │
│  What: How components interact                                  │
│  Who reads: Developers                                          │
│  Format: Sequence diagrams, state machines, data flows          │
│                                                                 │
│  Level 5: INTERFACE CONTRACTS                                   │
│  ────────────────────────────                                   │
│  What: Precise API specifications                               │
│  Who reads: Implementers                                        │
│  Format: API specs, interface definitions, schemas              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2.5 Phase 4: Validate

```
VALIDATE: Verify the design before implementation

Validation Techniques:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  1. ARCHITECTURE REVIEW                                         │
│     ├── Peer review by other architects                         │
│     ├── Stakeholder walkthrough                                 │
│     └── Checklist-based evaluation                              │
│                                                                 │
│  2. PROTOTYPING                                                 │
│     ├── Technical spikes for unknown areas                      │
│     ├── Integration proof-of-concepts                           │
│     └── Performance benchmarks                                  │
│                                                                 │
│  3. SCENARIO WALKTHROUGH                                        │
│     ├── Walk through key use cases                              │
│     ├── Trace data through the system                           │
│     └── Verify quality attribute scenarios                      │
│                                                                 │
│  4. RISK ASSESSMENT                                             │
│     ├── Identify technical risks                                │
│     ├── Assess probability and impact                           │
│     └── Define mitigation strategies                            │
│                                                                 │
│  5. TEAM VALIDATION                                             │
│     ├── Can the team build this?                                │
│     ├── Do they understand it?                                  │
│     └── Are they bought in?                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2.6 Phase 5: Communicate

```
COMMUNICATE: Architecture only has value if understood

Communication Principles:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  1. AUDIENCE-APPROPRIATE                                        │
│     ├── Executives: Focus on business value and risk            │
│     ├── Managers: Focus on timeline and resources               │
│     ├── Developers: Focus on technical details                  │
│     └── Operations: Focus on deployment and monitoring          │
│                                                                 │
│  2. VISUAL FIRST                                                │
│     ├── One diagram worth 1000 words                            │
│     ├── Use consistent notation                                 │
│     ├── Layer detail appropriately                              │
│     └── Keep diagrams up to date                                │
│                                                                 │
│  3. LIVING DOCUMENTATION                                        │
│     ├── Documentation that changes with the system              │
│     ├── Version controlled alongside code                       │
│     ├── Automated where possible                                │
│     └── Regular reviews and updates                             │
│                                                                 │
│  4. DECISION RECORDS                                            │
│     ├── Record decisions, not just outcomes                     │
│     ├── Include context and constraints at time of decision     │
│     ├── Note alternatives considered                            │
│     └── Update status as things change                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Architecture Artifacts

### 3.1 Document Hierarchy

```
+------------------------------------------------------------------+
|                    ARCHITECTURE DOCUMENT SET                      |
+------------------------------------------------------------------+
|                                                                   |
|  ┌───────────────────────────────────────────────────────────┐   |
|  │                                                           │   |
|  │  01. VISION & CONSTRAINTS                                 │   |
|  │      Purpose: Define scope and boundaries                 │   |
|  │      Contents:                                            │   |
|  │      - Project vision and goals                           │   |
|  │      - Success criteria                                   │   |
|  │      - Hard constraints                                   │   |
|  │      - MoSCoW prioritization                              │   |
|  │      - Stakeholder map                                    │   |
|  │                                                           │   |
|  └───────────────────────────────────────────────────────────┘   |
|                            │                                      |
|                            v                                      |
|  ┌───────────────────────────────────────────────────────────┐   |
|  │                                                           │   |
|  │  02. DOMAIN MODEL                                         │   |
|  │      Purpose: Define the problem space                    │   |
|  │      Contents:                                            │   |
|  │      - Core entities and value objects                    │   |
|  │      - Entity relationships                               │   |
|  │      - Aggregate boundaries                               │   |
|  │      - Domain services                                    │   |
|  │      - Glossary/Ubiquitous language                       │   |
|  │                                                           │   |
|  └───────────────────────────────────────────────────────────┘   |
|                            │                                      |
|                            v                                      |
|  ┌───────────────────────────────────────────────────────────┐   |
|  │                                                           │   |
|  │  03. TECHNOLOGY-SPECIFIC CONCERNS                         │   |
|  │      Purpose: Address system-specific challenges          │   |
|  │      Examples:                                            │   |
|  │      - Concurrency model (for concurrent systems)         │   |
|  │      - Data model (for data-intensive systems)            │   |
|  │      - Security model (for sensitive systems)             │   |
|  │      - Distribution model (for distributed systems)       │   |
|  │                                                           │   |
|  └───────────────────────────────────────────────────────────┘   |
|                            │                                      |
|                            v                                      |
|  ┌───────────────────────────────────────────────────────────┐   |
|  │                                                           │   |
|  │  04. ARCHITECTURE VIEWS (C4)                              │   |
|  │      Purpose: Show system structure                       │   |
|  │      Contents:                                            │   |
|  │      - System context (Level 1)                           │   |
|  │      - Container diagram (Level 2)                        │   |
|  │      - Component diagrams (Level 3)                       │   |
|  │      - Code/class diagrams (Level 4, selective)           │   |
|  │                                                           │   |
|  └───────────────────────────────────────────────────────────┘   |
|                            │                                      |
|                            v                                      |
|  ┌───────────────────────────────────────────────────────────┐   |
|  │                                                           │   |
|  │  05. INTERFACE CONTRACTS                                  │   |
|  │      Purpose: Define integration points                   │   |
|  │      Contents:                                            │   |
|  │      - Public API specifications                          │   |
|  │      - Internal service contracts                         │   |
|  │      - Event/message schemas                              │   |
|  │      - Error handling contracts                           │   |
|  │                                                           │   |
|  └───────────────────────────────────────────────────────────┘   |
|                            │                                      |
|                            v                                      |
|  ┌───────────────────────────────────────────────────────────┐   |
|  │                                                           │   |
|  │  06. BEHAVIORAL SPECIFICATIONS                            │   |
|  │      Purpose: Define how the system behaves               │   |
|  │      Contents:                                            │   |
|  │      - State machines                                     │   |
|  │      - Sequence diagrams                                  │   |
|  │      - Data flow diagrams                                 │   |
|  │      - Activity diagrams                                  │   |
|  │                                                           │   |
|  └───────────────────────────────────────────────────────────┘   |
|                            │                                      |
|                            v                                      |
|  ┌───────────────────────────────────────────────────────────┐   |
|  │                                                           │   |
|  │  07. ARCHITECTURE DECISION RECORDS (ADRs)                 │   |
|  │      Purpose: Capture decisions and rationale             │   |
|  │      Contents:                                            │   |
|  │      - Decision title                                     │   |
|  │      - Context and constraints                            │   |
|  │      - Options considered                                 │   |
|  │      - Decision and rationale                             │   |
|  │      - Consequences                                       │   |
|  │                                                           │   |
|  └───────────────────────────────────────────────────────────┘   |
|                                                                   |
+------------------------------------------------------------------+
```

### 3.2 When to Create Each Document

```
+------------------------------------------------------------------+
|                    DOCUMENT TIMING GUIDE                          |
+------------------------------------------------------------------+
|                                                                   |
|  Project Phase        Documents to Create                         |
|  ─────────────        ─────────────────────                       |
|                                                                   |
|  INCEPTION            01. Vision & Constraints (draft)            |
|  (Week 1-2)           02. Domain Model (initial)                  |
|                                                                   |
|  ELABORATION          01. Vision & Constraints (final)            |
|  (Week 2-4)           02. Domain Model (detailed)                 |
|                       03. Technology-Specific Concerns            |
|                       04. Architecture Views (L1, L2)             |
|                       07. Initial ADRs                            |
|                                                                   |
|  CONSTRUCTION         04. Architecture Views (L3)                 |
|  (Ongoing)            05. Interface Contracts                     |
|                       06. Behavioral Specs (as needed)            |
|                       07. Additional ADRs                         |
|                                                                   |
|  TRANSITION           All documents reviewed and updated          |
|  (Pre-release)        Operations documentation                    |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 4. Design Principles

### 4.1 Fundamental Principles

```
+------------------------------------------------------------------+
|                    FUNDAMENTAL PRINCIPLES                         |
+------------------------------------------------------------------+
|                                                                   |
|  SEPARATION OF CONCERNS                                           |
|  ─────────────────────                                            |
|  Each component should have one reason to change                  |
|  │                                                                |
|  ├── Separate by layer (presentation, business, data)             |
|  ├── Separate by feature (user, order, payment)                   |
|  └── Separate by rate of change (stable core, volatile edge)      |
|                                                                   |
|                                                                   |
|  INFORMATION HIDING                                               |
|  ────────────────────                                             |
|  Implementation details should be hidden behind interfaces        |
|  │                                                                |
|  ├── Expose WHAT, hide HOW                                        |
|  ├── Changes should not ripple beyond boundaries                  |
|  └── Depend on abstractions, not concretions                      |
|                                                                   |
|                                                                   |
|  DESIGN FOR CHANGE                                                |
|  ────────────────────                                             |
|  Assume requirements will change                                  |
|  │                                                                |
|  ├── Identify volatile areas                                      |
|  ├── Encapsulate variation                                        |
|  └── Make change easier than no change                            |
|                                                                   |
|                                                                   |
|  DEFER COMMITMENT                                                 |
|  ──────────────────                                               |
|  Make decisions at the last responsible moment                    |
|  │                                                                |
|  ├── Gather more information first                                |
|  ├── Make decisions reversible where possible                     |
|  └── Avoid premature optimization                                 |
|                                                                   |
|                                                                   |
|  SIMPLE OVER COMPLEX                                              |
|  ─────────────────────                                            |
|  Complexity is the enemy of reliability                           |
|  │                                                                |
|  ├── If you don't need it, don't build it                         |
|  ├── The best code is no code                                     |
|  └── Complexity grows faster than functionality                   |
|                                                                   |
+------------------------------------------------------------------+
```

### 4.2 Quality Attribute Trade-offs

```
+------------------------------------------------------------------+
|                  QUALITY ATTRIBUTE TRADE-OFFS                     |
+------------------------------------------------------------------+
|                                                                   |
|  Common Trade-off Pairs:                                          |
|                                                                   |
|  PERFORMANCE vs MAINTAINABILITY                                   |
|  ─────────────────────────────                                    |
|  Optimization often increases complexity                          |
|  → Profile before optimizing                                      |
|  → Optimize hot paths only                                        |
|                                                                   |
|  SECURITY vs USABILITY                                            |
|  ───────────────────────                                          |
|  More security often means more friction                          |
|  → Risk-based security decisions                                  |
|  → Make secure paths easy                                         |
|                                                                   |
|  FLEXIBILITY vs SIMPLICITY                                        |
|  ─────────────────────────                                        |
|  More options means more complexity                               |
|  → YAGNI (You Aren't Gonna Need It)                               |
|  → Prefer conventions over configuration                          |
|                                                                   |
|  CONSISTENCY vs AVAILABILITY                                      |
|  ───────────────────────────                                      |
|  CAP theorem in distributed systems                               |
|  → Choose based on domain requirements                            |
|  → Understand what "eventual" means                               |
|                                                                   |
|  TIME-TO-MARKET vs TECHNICAL DEBT                                 |
|  ───────────────────────────────                                  |
|  Shortcuts now mean pain later                                    |
|  → Make debt visible                                              |
|  → Budget for debt repayment                                      |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 5. Architecture Decision Records (ADRs)

### 5.1 ADR Template

```markdown
# ADR-NNN: [Title]

## Status
[Proposed | Accepted | Deprecated | Superseded by ADR-XXX]

## Date
YYYY-MM-DD

## Context
What is the issue we're facing? What are the forces at play?

## Decision Drivers
- Driver 1
- Driver 2
- Driver 3

## Considered Options
1. Option A
2. Option B
3. Option C

## Decision
We will use Option B because...

## Consequences

### Positive
- Benefit 1
- Benefit 2

### Negative
- Drawback 1
- Drawback 2

### Risks
- Risk 1: Mitigation
- Risk 2: Mitigation

## Related Decisions
- ADR-XXX: Related decision
```

### 5.2 When to Write an ADR

```
Write an ADR when:

✓ Choosing between significant alternatives
✓ Making decisions that affect multiple components
✓ Introducing new technologies or patterns
✓ Deviating from established conventions
✓ Making decisions that are difficult to reverse
✓ Decisions that team members question

Don't write an ADR for:

✗ Standard library choices
✗ Obvious implementations
✗ Decisions easily changed
✗ Team conventions already documented elsewhere
```

---

## 6. Common Pitfalls

### 6.1 Architecture Anti-Patterns

```
+------------------------------------------------------------------+
|                    ARCHITECTURE ANTI-PATTERNS                     |
+------------------------------------------------------------------+
|                                                                   |
|  IVORY TOWER ARCHITECTURE                                         |
|  ─────────────────────────                                        |
|  Symptom: Architecture designed in isolation from developers      |
|  Problem: Impractical, not followed, breeds resentment            |
|  Solution: Collaborative design, involve implementers             |
|                                                                   |
|  BIG DESIGN UP FRONT (BDUF)                                       |
|  ──────────────────────────                                       |
|  Symptom: Trying to design everything before building             |
|  Problem: Requirements change, design becomes obsolete            |
|  Solution: Iterative design, embrace change                       |
|                                                                   |
|  RESUME-DRIVEN DEVELOPMENT                                        |
|  ────────────────────────                                         |
|  Symptom: Choosing technology for CV, not problem                 |
|  Problem: Over-engineered, team can't maintain                    |
|  Solution: Boring technology, team capabilities                   |
|                                                                   |
|  ANALYSIS PARALYSIS                                               |
|  ──────────────────                                               |
|  Symptom: Endless discussion, no decision                         |
|  Problem: Nothing gets built                                      |
|  Solution: Time-box decisions, accept "good enough"               |
|                                                                   |
|  GOLDEN HAMMER                                                    |
|  ──────────────                                                   |
|  Symptom: Using same solution for every problem                   |
|  Problem: Square peg, round hole                                  |
|  Solution: Match solution to problem                              |
|                                                                   |
|  ARCHITECTURE ASTRONAUT                                           |
|  ────────────────────────                                         |
|  Symptom: Overly abstract, over-generalized                       |
|  Problem: Simple things become complex                            |
|  Solution: Solve today's problem, not tomorrow's                  |
|                                                                   |
+------------------------------------------------------------------+
```

### 6.2 Warning Signs

```
Your architecture might have problems if:

❌ No one can explain it simply
❌ New team members take weeks to understand it
❌ Simple changes require touching many files
❌ The design docs don't match the code
❌ The team dreads making changes
❌ "Technical debt" is mentioned frequently
❌ Performance optimizations are everywhere
❌ Everything depends on everything else
❌ Tests are slow or flaky
❌ Deployment is painful or scary
```

---

## 7. Continuous Architecture

### 7.1 Architecture Fitness Functions

```
+------------------------------------------------------------------+
|                   ARCHITECTURE FITNESS FUNCTIONS                  |
+------------------------------------------------------------------+
|                                                                   |
|  Fitness functions are objective measures of architecture health  |
|                                                                   |
|  STRUCTURAL FITNESS                                               |
|  ───────────────────                                              |
|  - Cyclic dependencies: Should be 0                               |
|  - Module coupling: Should be decreasing                          |
|  - Layer violations: Should be 0                                  |
|  - Component size: Should be within bounds                        |
|                                                                   |
|  PERFORMANCE FITNESS                                              |
|  ─────────────────────                                            |
|  - P99 latency: Should be under target                            |
|  - Throughput: Should meet baseline                               |
|  - Memory usage: Should be within bounds                          |
|  - CPU utilization: Should be under threshold                     |
|                                                                   |
|  SECURITY FITNESS                                                 |
|  ─────────────────                                                |
|  - Known vulnerabilities: Should be 0 critical                    |
|  - Auth coverage: Should be 100%                                  |
|  - Secrets in code: Should be 0                                   |
|  - Security headers: Should all be present                        |
|                                                                   |
|  RELIABILITY FITNESS                                              |
|  ───────────────────                                              |
|  - Test coverage: Should be above threshold                       |
|  - Error rate: Should be below threshold                          |
|  - Recovery time: Should be under target                          |
|  - Uptime: Should meet SLA                                        |
|                                                                   |
|  Run these automatically in CI/CD pipeline                        |
|                                                                   |
+------------------------------------------------------------------+
```

### 7.2 Architecture Review Cadence

```
+------------------------------------------------------------------+
|                    ARCHITECTURE REVIEW CADENCE                    |
+------------------------------------------------------------------+
|                                                                   |
|  CONTINUOUS (Automated)                                           |
|  ───────────────────────                                          |
|  - Dependency checks in CI                                        |
|  - Architecture fitness functions                                 |
|  - Security scanning                                              |
|  - Performance baseline tests                                     |
|                                                                   |
|  WEEKLY (Team)                                                    |
|  ─────────────                                                    |
|  - Review significant PRs for architecture impact                 |
|  - Discuss emerging patterns                                      |
|  - Address technical debt                                         |
|                                                                   |
|  MONTHLY (Cross-team)                                             |
|  ────────────────────                                             |
|  - Architecture sync across teams                                 |
|  - Review ADRs from past month                                    |
|  - Identify shared concerns                                       |
|                                                                   |
|  QUARTERLY (Strategic)                                            |
|  ─────────────────────                                            |
|  - Architecture roadmap review                                    |
|  - Technology radar update                                        |
|  - Major evolution planning                                       |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 8. Summary: The Architect's Checklist

```
+------------------------------------------------------------------+
|                    THE ARCHITECT'S CHECKLIST                      |
+------------------------------------------------------------------+
|                                                                   |
|  BEFORE DESIGNING                                                 |
|  □ Understand the problem deeply                                  |
|  □ Identify stakeholders and their concerns                       |
|  □ Clarify constraints (technical, org, regulatory)               |
|  □ Define quality attribute priorities                            |
|  □ Identify architecturally significant requirements              |
|                                                                   |
|  WHILE DESIGNING                                                  |
|  □ Generate multiple options                                      |
|  □ Evaluate options against requirements                          |
|  □ Document decisions and rationale (ADRs)                        |
|  □ Design for change                                              |
|  □ Keep it as simple as possible                                  |
|  □ Involve the team                                               |
|                                                                   |
|  AFTER DESIGNING                                                  |
|  □ Validate with prototypes/spikes                                |
|  □ Review with stakeholders                                       |
|  □ Communicate effectively to all audiences                       |
|  □ Keep documentation living                                      |
|  □ Monitor architecture fitness                                   |
|  □ Evolve continuously                                            |
|                                                                   |
+------------------------------------------------------------------+

"The goal of architecture is to identify the design decisions
 that matter most, make them early, and make them well."
                                        - Grady Booch
```
