# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| `main` branch | ✅ Yes |
| Older releases | ❌ No (upgrade recommended) |

## Reporting a Vulnerability

**Please do NOT open a public GitHub issue for security vulnerabilities.**

Report security issues privately via email:

**support@injoys.ai**

Include in your report:
- Description of the vulnerability
- Steps to reproduce
- Potential impact (data exposure, authentication bypass, etc.)
- Any suggested fix if you have one

### What to Expect

- **Acknowledgement**: within 48 hours
- **Status update**: within 7 days
- **Resolution target**: within 30 days for critical issues

We will credit you in the release notes unless you prefer to remain anonymous.

## Scope

In scope:
- `gateway/` (Rust Gateway service)
- Authentication and token handling
- Database credential management
- Matterbridge integration endpoints

Out of scope:
- Vulnerabilities in third-party dependencies (report upstream)
- Social engineering attacks
- Issues requiring physical access
