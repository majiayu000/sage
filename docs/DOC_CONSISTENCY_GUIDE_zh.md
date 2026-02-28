# Documentation Consistency Guide

A practical guide for maintaining consistency between English and Chinese documentation.

## 🎯 Overview

This project maintains documentation in both English and Chinese. This guide provides tools and processes to ensure both versions stay synchronized and consistent.

## 📋 Current Status

### File Pairs
- `README.md` ↔ `README_zh.md` ✅
- `docs/README.md` ↔ `docs/README_zh.md` ❌ (Missing Chinese)
- `docs/user-guide/README.md` ↔ `docs/user-guide/README_zh.md` ❌ (Missing Chinese)
- `docs/development/README.md` ↔ `docs/development/README_zh.md` ❌ (Missing Chinese)

## 🛠️ Tools

### 1. Consistency Checker Script

Run the automated consistency checker:

```bash
# Check documentation consistency
python3 scripts/check_doc_consistency.py

# This will:
# - Find missing translation files
# - Check if files are up-to-date
# - Verify structural consistency
# - Generate a detailed report
```

### 2. GitHub Actions

Automated checks run on:
- Pull requests affecting documentation
- Pushes to main branch
- Daily scheduled runs

The workflow will:
- Run consistency checks
- Comment on PRs with results
- Create issues for problems
- Upload detailed reports

### 3. Manual Checks

Quick manual verification:

```bash
# Count lines in both README files
wc -l README.md README_zh.md

# Check last modification times
ls -la README*.md

# Compare file structures
find docs -name "*.md" | sort
find docs -name "*_zh.md" | sort
```

## 📝 Workflow

### When Updating English Documentation

1. **Update English file**
   ```bash
   # Edit the English documentation
   vim docs/user-guide/getting-started.md
   
   # Commit with special tag
   git commit -m "docs: update getting started guide [needs-zh-update]"
   ```

2. **Check consistency**
   ```bash
   python3 scripts/check_doc_consistency.py
   ```

3. **Update Chinese counterpart**
   ```bash
   # Update or create Chinese version
   vim docs/user-guide/getting-started_zh.md
   
   # Commit referencing English update
   git commit -m "docs(zh): update getting started guide (sync with abc123)"
   ```

### When Adding New Documentation

1. **Create English version first**
   ```bash
   # Create new documentation file
   vim docs/api/new-feature.md
   git add docs/api/new-feature.md
   git commit -m "docs: add new feature documentation [needs-zh-version]"
   ```

2. **Create Chinese version**
   ```bash
   # Create corresponding Chinese file
   vim docs/api/new-feature_zh.md
   git add docs/api/new-feature_zh.md
   git commit -m "docs(zh): add new feature documentation"
   ```

## 🔍 Quality Checks

### Structure Consistency
- Both versions should have same number of headers
- Code blocks should match between versions
- Links and references should be equivalent
- Images and diagrams should be shared

### Content Consistency
- Technical accuracy maintained
- All features documented in both languages
- Examples and code samples identical
- Version information synchronized

### Maintenance
- Regular consistency checks (automated)
- Update outdated translations
- Fix structural mismatches
- Maintain terminology consistency

## 📊 Monitoring

### Automated Reports
- Daily consistency reports via GitHub Actions
- PR comments with consistency status
- Issue creation for problems
- Artifact uploads for detailed analysis

### Manual Monitoring
- Weekly review of consistency status
- Monthly audit of documentation quality
- Quarterly review of processes and tools

## 🚀 Best Practices

### For Documentation Authors
1. **Always update English first** - English is the source of truth
2. **Use clear commit messages** - Include `[needs-zh-update]` tags
3. **Check consistency** - Run checker before committing
4. **Maintain structure** - Keep same heading structure across languages

### For Maintainers
1. **Monitor consistency reports** - Review automated reports regularly
2. **Prioritize updates** - Focus on user-facing documentation first
3. **Coordinate updates** - Ensure timely Chinese updates
4. **Quality assurance** - Regular manual reviews

### For Contributors
1. **Follow workflow** - Use established processes
2. **Check before submitting** - Run consistency checker
3. **Document changes** - Clear commit messages and PR descriptions
4. **Coordinate with team** - Communicate about documentation changes

## 🔧 Configuration

### Script Configuration
The consistency checker can be configured by modifying `scripts/check_doc_consistency.py`:

```python
# Adjust these settings as needed
FRESHNESS_THRESHOLD = 86400  # 24 hours in seconds
IGNORE_PATTERNS = ['.git', 'node_modules', 'target']
REQUIRED_PAIRS = ['README.md', 'docs/README.md']
```

### GitHub Actions Configuration
Modify `.github/workflows/doc-consistency.yml` to:
- Change check frequency
- Adjust notification settings
- Customize report format
- Add additional checks

## 📞 Support

### Getting Help
- Run `python3 scripts/check_doc_consistency.py --help`
- Check GitHub Actions logs for detailed error information
- Create issues with `documentation` label for problems
- Contact documentation maintainers for guidance

### Common Issues
1. **Missing Chinese files** - Create corresponding `*_zh.md` files
2. **Outdated translations** - Update Chinese files to match English
3. **Structure mismatches** - Ensure same heading structure
4. **Script errors** - Check Python version and file permissions

---

**Last Updated**: 2025-01-15  
**Maintainer**: Documentation Team  
**Review Schedule**: Monthly
