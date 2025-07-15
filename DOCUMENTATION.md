# Sage Agent Documentation Guide

Quick navigation guide for all Sage Agent documentation.

## 🚀 Getting Started

### New Users
1. **[Installation Guide](docs/user-guide/getting-started.md#installation)** - Install Sage Agent
2. **[Basic Configuration](docs/user-guide/configuration.md#basic-setup)** - Set up your first config
3. **[First Task](docs/user-guide/getting-started.md#first-task)** - Run your first agent task
4. **[CLI Reference](docs/user-guide/cli-reference.md)** - Learn the command-line interface

### Developers
1. **[Development Setup](docs/development/setup.md)** - Set up development environment
2. **[Architecture Overview](docs/architecture/system-overview.md)** - Understand the system
3. **[Contributing Guide](docs/development/contributing.md)** - How to contribute
4. **[API Reference](docs/api/)** - Detailed API documentation

## 📚 Documentation Structure

```
docs/
├── README.md                    # Documentation overview
├── user-guide/                  # End-user documentation
│   ├── getting-started.md       # Installation and first steps
│   ├── configuration.md         # Configuration guide
│   ├── cli-reference.md         # CLI command reference
│   ├── sdk-usage.md             # SDK programming guide
│   ├── tools-reference.md       # Available tools reference
│   └── troubleshooting.md       # Common issues and solutions
├── architecture/                # System architecture
│   ├── system-overview.md       # High-level architecture
│   ├── agent-execution.md       # Agent execution model
│   ├── tool-system.md           # Tool system design
│   ├── llm-integration.md       # LLM integration architecture
│   ├── configuration.md         # Configuration system
│   └── ui-components.md         # UI component architecture
├── development/                 # Developer documentation
│   ├── setup.md                 # Development environment setup
│   ├── contributing.md          # Contribution guidelines
│   ├── code-style.md            # Code style and conventions
│   ├── testing.md               # Testing guidelines
│   ├── MCP_INTEGRATION_PLAN.md  # MCP integration plan
│   ├── TOOLS_EXPANSION_PLAN.md  # Tool expansion roadmap
│   └── release-process.md       # Release management
├── api/                         # API reference documentation
│   ├── core-api.md              # sage-core API reference
│   ├── sdk-api.md               # sage-sdk API reference
│   ├── tools-api.md             # sage-tools API reference
│   └── cli-api.md               # sage-cli API reference
└── planning/                    # Project planning
    ├── TODO.md                  # Chinese TODO list
    ├── TODO_EN.md               # English TODO list
    ├── roadmap.md               # Project roadmap
    └── adr/                     # Architecture Decision Records
        ├── README.md            # ADR index
        ├── template.md          # ADR template
        └── 001-*.md             # Individual ADRs
```

## 🎯 Documentation by Role

### End Users
- **Getting Started**: [Installation](docs/user-guide/getting-started.md) → [Configuration](docs/user-guide/configuration.md) → [First Task](docs/user-guide/getting-started.md#first-task)
- **Daily Usage**: [CLI Reference](docs/user-guide/cli-reference.md) → [Tools Reference](docs/user-guide/tools-reference.md)
- **Advanced Usage**: [SDK Guide](docs/user-guide/sdk-usage.md) → [Configuration Advanced](docs/user-guide/configuration.md#advanced-settings)
- **Troubleshooting**: [Common Issues](docs/user-guide/troubleshooting.md) → [Error Messages](docs/user-guide/troubleshooting.md#error-messages)

### Developers
- **Getting Started**: [Dev Setup](docs/development/setup.md) → [Architecture](docs/architecture/system-overview.md) → [Contributing](docs/development/contributing.md)
- **Core Development**: [Code Style](docs/development/code-style.md) → [Testing](docs/development/testing.md) → [API Reference](docs/api/)
- **Tool Development**: [Tool System](docs/architecture/tool-system.md) → [Tools API](docs/api/tools-api.md) → [Tool Examples](examples/)
- **Architecture**: [System Overview](docs/architecture/system-overview.md) → [ADRs](docs/planning/adr/) → [Planning](docs/planning/)

### Contributors
- **First Contribution**: [Contributing Guide](docs/development/contributing.md) → [Development Setup](docs/development/setup.md)
- **Finding Tasks**: [TODO Lists](docs/planning/) → [Issue Tracker](https://github.com/your-org/sage-agent/issues)
- **Code Quality**: [Code Style](docs/development/code-style.md) → [Testing Guide](docs/development/testing.md)
- **Architecture Changes**: [ADR Process](docs/planning/adr/README.md) → [ADR Template](docs/planning/adr/template.md)

### Maintainers
- **Release Management**: [Release Process](docs/development/release-process.md) → [Versioning](docs/development/release-process.md#versioning)
- **Planning**: [Roadmap](docs/planning/roadmap.md) → [TODO Management](docs/planning/) → [Milestones](docs/planning/milestones.md)
- **Architecture**: [ADR Reviews](docs/planning/adr/) → [Architecture Docs](docs/architecture/)

## 🔍 Finding Information

### By Topic
- **Installation**: [User Guide → Getting Started](docs/user-guide/getting-started.md)
- **Configuration**: [User Guide → Configuration](docs/user-guide/configuration.md)
- **Tools**: [User Guide → Tools Reference](docs/user-guide/tools-reference.md) + [API → Tools API](docs/api/tools-api.md)
- **SDK**: [User Guide → SDK Usage](docs/user-guide/sdk-usage.md) + [API → SDK API](docs/api/sdk-api.md)
- **Architecture**: [Architecture Documentation](docs/architecture/)
- **Contributing**: [Development → Contributing](docs/development/contributing.md)
- **Planning**: [Planning Documentation](docs/planning/)

### By Format
- **Tutorials**: [User Guide](docs/user-guide/) - Step-by-step guides
- **Reference**: [API Documentation](docs/api/) - Detailed API reference
- **Explanations**: [Architecture](docs/architecture/) - How and why things work
- **How-to Guides**: [Development](docs/development/) - Specific tasks and procedures

## 📝 Documentation Standards

### Writing Style
- **Clear and Concise**: Use simple, direct language
- **User-Focused**: Write from the user's perspective
- **Example-Rich**: Include practical examples
- **Up-to-Date**: Keep documentation current with code

### Structure
- **Consistent Format**: Follow established templates
- **Logical Flow**: Organize information logically
- **Cross-References**: Link related information
- **Navigation**: Provide clear navigation paths

### Maintenance
- **Regular Updates**: Update with code changes
- **Review Process**: Review documentation changes
- **Feedback Integration**: Incorporate user feedback
- **Quality Checks**: Ensure accuracy and completeness

## 🚀 Contributing to Documentation

### How to Help
1. **Report Issues**: Found outdated or incorrect information? [Create an issue](https://github.com/your-org/sage-agent/issues)
2. **Suggest Improvements**: Ideas for better documentation? [Start a discussion](https://github.com/your-org/sage-agent/discussions)
3. **Submit Changes**: Ready to contribute? [Follow the contributing guide](docs/development/contributing.md)
4. **Translate**: Help translate documentation to other languages

### Documentation Types Needed
- **Tutorials**: Step-by-step learning guides
- **How-to Guides**: Problem-solving guides
- **Reference**: Technical reference material
- **Explanations**: Conceptual explanations

---

**Need help?** Check the [troubleshooting guide](docs/user-guide/troubleshooting.md) or [create an issue](https://github.com/your-org/sage-agent/issues).
