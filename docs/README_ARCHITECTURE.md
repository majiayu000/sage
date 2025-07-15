# README Architecture Guide

This document describes the new multi-language README architecture for Sage Agent.

## 📁 File Structure

```
sage-agent/
├── README_ENTRY.md          # Language selection entry point
├── README.md                # English documentation
├── README_zh.md             # Chinese documentation
├── docs/
│   └── language-selector.html  # Interactive language selector
└── scripts/
    └── sync_readme.py       # README synchronization checker
```

## 🌐 Language Selection System

### Entry Point (`README_ENTRY.md`)
- **Purpose**: Provides a clean language selection interface
- **Features**: 
  - Visual language buttons
  - Quick preview of installation commands
  - Project badges and status
- **Usage**: Can be used as the main README for GitHub display

### Language-Specific READMEs
- **`README.md`**: Complete English documentation
- **`README_zh.md`**: Complete Chinese documentation
- **Features**:
  - Language switcher at the top
  - Consistent structure and sections
  - Localized content and examples

### Interactive Selector (`docs/language-selector.html`)
- **Purpose**: Beautiful HTML page for language selection
- **Features**:
  - Responsive design
  - Animated buttons
  - Project branding
- **Usage**: Can be hosted or used locally

## 🔧 Maintenance Tools

### README Synchronization Script (`scripts/sync_readme.py`)

**Purpose**: Ensures consistency between English and Chinese README files.

**Features**:
- Structure comparison between language versions
- Section mapping with translation awareness
- Missing section detection
- Language link validation

**Usage**:
```bash
# Run synchronization check
python3 scripts/sync_readme.py

# Or use Makefile
make readme-sync
```

**Translation Mapping**: The script includes intelligent mapping between English and Chinese section names:
- "Features" ↔ "特性"
- "Quick Start" ↔ "快速开始"
- "Architecture" ↔ "架构"
- And many more...

## 📋 Content Guidelines

### Structure Consistency
Both language versions should maintain:
1. **Same section order**
2. **Equivalent content depth**
3. **Consistent formatting**
4. **Similar examples (localized)**

### Language-Specific Adaptations
- **Examples**: Use appropriate language in code comments
- **Commands**: Localize command descriptions
- **Links**: Point to language-appropriate resources when available
- **Cultural Context**: Adapt explanations for target audience

### Visual Elements
- **Badges**: Use consistent styling across languages
- **Emojis**: Maintain same emoji usage for visual consistency
- **Tables**: Keep same structure, translate content
- **Code Blocks**: Localize comments, keep code identical

## 🚀 Implementation Benefits

### For Users
- **Clear Language Choice**: Immediate language selection
- **Native Experience**: Full documentation in preferred language
- **Easy Switching**: Quick navigation between languages
- **Consistent Quality**: Maintained synchronization between versions

### For Maintainers
- **Automated Checks**: Script validates consistency
- **Clear Structure**: Organized file hierarchy
- **Easy Updates**: Makefile integration for checks
- **Quality Assurance**: Prevents documentation drift

## 🔄 Workflow

### Adding New Content
1. **Update English README** with new content
2. **Update Chinese README** with translated content
3. **Run sync check**: `make readme-sync`
4. **Fix any issues** reported by the script
5. **Commit changes** for both language versions

### Regular Maintenance
1. **Weekly sync checks** during development
2. **Pre-release validation** before major releases
3. **Community feedback** integration for both languages
4. **Continuous improvement** of translation mappings

## 🎯 Future Enhancements

### Planned Features
- **Automated Translation Suggestions**: AI-assisted translation hints
- **Content Diff Visualization**: Visual comparison of language versions
- **Multi-Language Support**: Framework for additional languages
- **Integration Testing**: Automated README validation in CI/CD

### Community Contributions
- **Translation Improvements**: Community-driven translation refinements
- **New Language Additions**: Support for additional languages
- **Tool Enhancements**: Improvements to synchronization scripts
- **Documentation Feedback**: User experience improvements

## 📞 Support

For questions about the README architecture:
- **Issues**: Create GitHub issues for bugs or suggestions
- **Discussions**: Use GitHub Discussions for questions
- **Contributions**: Follow the contributing guidelines
- **Feedback**: Provide feedback on documentation quality

---

This architecture ensures that Sage Agent provides an excellent documentation experience for users in both English and Chinese, while maintaining consistency and quality across all language versions.
