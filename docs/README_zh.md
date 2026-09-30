# Sage Agent Documentation

本目录仅链接当前版本中实际存在的文件；未附链接的参考主题暂时没有独立指南。

Welcome to the Sage Agent documentation! This directory contains comprehensive documentation for developers, users, and contributors.

## 📚 Documentation Structure

### 📖 User Guide (`user-guide/`)
Documentation for end users of Sage Agent:
- **[Getting Started](user-guide/installation.md)** - Installation and basic usage
- **[Configuration Guide](user-guide/configuration.md)** - Configuration options and examples
- **CLI Reference** - Command-line interface documentation
- **SDK Usage** - Programmatic usage with the SDK
- **Tools Reference** - Available tools and their usage
- **Troubleshooting** - Common issues and solutions

### 🏗️ Architecture (`architecture/`)
System design and architecture documentation:
- **System Overview** - High-level architecture
- **Agent Execution Model** - How agents work
- **Tool System** - Tool architecture and design
- **LLM Integration** - Language model integration
- **Configuration System** - Configuration architecture
- **UI Components** - User interface design

### 🔧 Development (`development/`)
Documentation for developers and contributors:
- **Development Setup** - Setting up development environment
- **[Contributing Guide](../CONTRIBUTING.md)** - How to contribute to the project
- **Code Style Guide** - Coding standards and conventions
- **Testing Guide** - Testing strategies and practices
- **[MCP Integration Plan](development/MCP_INTEGRATION_PLAN.md)** - Model Context Protocol integration
- **[Tools Expansion Plan](development/TOOLS_EXPANSION_PLAN.md)** - Tool ecosystem expansion
- **[Release Process](development/release-process.md)** - How releases are managed
- **[Release Gates](development/release-process.md#required-gates)** - Required CI, security, artifact, and supply-chain gates

### 📋 Planning (`planning/`)
Project planning and roadmap documentation:
- **[TODO List (中文)](planning/TODO.md)** - Chinese version of TODO items
- **[TODO List (English)](planning/TODO_EN.md)** - English version of TODO items
- **Roadmap** - Project roadmap and milestones
- **[Architecture Decisions](planning/adr/)** - Architecture Decision Records

### 🔌 API Reference (`api/`)
API documentation and references:
- **Core API** - sage-core crate API reference
- **SDK API** - sage-sdk crate API reference
- **Tools API** - sage-tools crate API reference
- **CLI API** - sage-cli crate API reference

## 🚀 Quick Start

### For Users
1. Start with [Getting Started](user-guide/installation.md)
2. Review [Configuration Guide](user-guide/configuration.md)
3. Explore Tools Reference

### For Developers
1. Read Development Setup
2. Review [Contributing Guide](../CONTRIBUTING.md)
3. Check Architecture Overview

### For Contributors
1. Review [Contributing Guide](../CONTRIBUTING.md)
2. Check [TODO Lists](planning/) for available tasks
3. Read Code Style Guide

## 📝 Documentation Standards

### Writing Guidelines
- Use clear, concise language
- Include code examples where appropriate
- Keep documentation up-to-date with code changes
- Use consistent formatting and structure

### File Organization
- Use descriptive filenames with kebab-case
- Include README.md in each directory
- Cross-reference related documents
- Maintain a logical hierarchy

### Code Examples
- Provide working, tested examples
- Include both basic and advanced usage
- Add comments to explain complex concepts
- Use realistic scenarios and data

## 🔄 Maintenance

This documentation is maintained by the Sage Agent team and community contributors. 

### Updating Documentation
- Documentation should be updated with each code change
- Use pull requests for documentation changes
- Review documentation for accuracy and clarity
- Update cross-references when moving or renaming files

### Feedback and Contributions
- Report documentation issues via GitHub issues
- Suggest improvements through pull requests
- Help translate documentation to other languages
- Share usage examples and best practices

---

**Note**: This documentation is continuously updated. For the latest information, always refer to the main branch of the repository.
