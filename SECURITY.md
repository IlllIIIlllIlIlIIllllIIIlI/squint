# Security Policy

## Supported Versions

Only the latest released version receives security fixes.

| Version | Supported |
|---|---|
| latest | yes |
| older  | no  |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities privately via [GitHub Security Advisories](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/security/advisories/new).
If you prefer email, contact the maintainers directly — see the commit log for contact details.

Include as much of the following as possible:

- A description of the vulnerability and its potential impact
- Steps to reproduce or a minimal proof-of-concept
- Any suggested mitigations

## Response Timeline

| Step | Target |
|---|---|
| Acknowledgement | Within 3 business days |
| Initial assessment | Within 7 business days |
| Fix or mitigation | Best effort; depends on severity |

We will credit reporters in the release notes unless you prefer to remain anonymous.

## Scope

This project is a static analysis tool that reads SQL files from disk and writes
output to stdout. It does not:

- Accept network connections
- Execute SQL against any database
- Store or transmit user data

The primary attack surface is **malicious SQL input** passed to the lexer/parser.
If you find a way to cause a panic, memory safety issue, or excessive resource
consumption via crafted SQL input, that is in scope.
