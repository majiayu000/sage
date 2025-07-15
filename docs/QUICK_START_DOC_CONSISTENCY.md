# Quick Start: Documentation Consistency

A quick guide to using the documentation consistency tools.

## 🚀 Quick Commands

### Check Documentation Status
```bash
# Using Make (recommended)
make doc-status

# Manual check
wc -l README.md README_zh.md
```

### Run Consistency Check
```bash
# Using Make (recommended)
make doc-check

# Direct script execution
python3 scripts/check_doc_consistency.py
```

### View Results
The consistency checker will:
- ✅ Show files that are in sync
- ⚠️ Highlight outdated files
- ❌ Report missing files
- 📊 Generate a detailed report

## 📋 Common Workflows

### When You Update English Documentation

1. **Edit the English file**
   ```bash
   vim README.md
   # Make your changes
   ```

2. **Check what needs updating**
   ```bash
   make doc-check
   ```

3. **Update Chinese version if needed**
   ```bash
   vim README_zh.md
   # Update to match English changes
   ```

4. **Verify consistency**
   ```bash
   make doc-check
   # Should show ✅ all files in sync
   ```

### When Adding New Documentation

1. **Create English version first**
   ```bash
   vim docs/new-feature.md
   ```

2. **Create Chinese version**
   ```bash
   vim docs/new-feature_zh.md
   ```

3. **Verify both files are detected**
   ```bash
   make doc-check
   ```

## 🔍 Understanding the Output

### Status Indicators
- ✅ **Synced**: Files are up-to-date
- ⚠️ **Outdated**: Chinese file is older than English
- ❌ **Missing**: Chinese file doesn't exist
- 🔍 **Structure Issue**: Different number of headers/code blocks

### Example Output
```
🔍 Checking documentation consistency...

📊 Documentation Consistency Report
Generated: 2025-01-15T10:30:00
Status: ❌ FAIL
File pairs checked: 3
Total issues: 2

🔍 Issues Found:
  • Missing files: 1
  • Outdated files: 1
  • Structure issues: 0

📋 Detailed Issues:
  1. [MISSING_CHINESE] Chinese counterpart missing for docs/api/README.md
     File: docs/api/README_zh.md
  2. [OUTDATED_CHINESE] Chinese file may be outdated
     EN: docs/user-guide/README.md
     ZH: docs/user-guide/README_zh.md
```

## 🛠️ Troubleshooting

### Common Issues

**"python: command not found"**
```bash
# Use python3 instead
python3 scripts/check_doc_consistency.py
```

**"Permission denied"**
```bash
# Make script executable
chmod +x scripts/check_doc_consistency.py
```

**"No such file or directory"**
```bash
# Run from project root
cd /path/to/sage-agent
make doc-check
```

### Getting Help
- Check the full guide: [docs/DOC_CONSISTENCY_GUIDE.md](DOC_CONSISTENCY_GUIDE.md)
- View script help: `python3 scripts/check_doc_consistency.py --help`
- Create an issue with `documentation` label

## 📊 Automation

### GitHub Actions
The consistency check runs automatically on:
- Pull requests affecting documentation
- Pushes to main branch
- Daily at 9 AM UTC

### Local Development
Add to your development workflow:
```bash
# Before committing documentation changes
make doc-check

# Include in your regular checks
make quick  # runs fmt, clippy, test
make doc-check  # check documentation
```

## 🎯 Best Practices

1. **Always update English first** - It's the source of truth
2. **Check before committing** - Run `make doc-check`
3. **Keep structure consistent** - Same headers in both languages
4. **Update promptly** - Don't let Chinese docs get too outdated
5. **Use clear commit messages** - Help others understand changes

---

**Need more details?** See the full [Documentation Consistency Guide](DOC_CONSISTENCY_GUIDE.md)
