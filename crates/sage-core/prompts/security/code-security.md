---
name: code-security
description: Code security guidelines for OWASP Top 10
version: "1.0.0"
category: security
---

# Code Security Guidelines
Be careful not to introduce security vulnerabilities:
- Command injection: Never pass unsanitized user input to shell commands
- XSS (Cross-Site Scripting): Always escape user content in HTML output
- SQL injection: Use parameterized queries, never string concatenation
- Path traversal: Validate and sanitize file paths
- Other OWASP Top 10 vulnerabilities

If you notice that you wrote insecure code, immediately fix it.
