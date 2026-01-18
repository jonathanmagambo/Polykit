# Security Policy

## Supported Versions

Polykit is currently in active development. Security updates are provided for the latest release version.

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :x:                |
| < 0.1   | :x:                |

**Note:** As Polykit is pre-1.0, we recommend always using the latest version. Once version 1.0 is released, we will provide long-term support for stable versions.

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security vulnerability in Polykit, please follow these steps:

### How to Report

1. **Do not** open a public GitHub issue for security vulnerabilities.
2. Email security details to the maintainers or create a private security advisory on GitHub.
3. Include as much information as possible:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)
   - Affected versions

### What to Expect

- **Initial Response:** You will receive an acknowledgment within 48 hours.
- **Status Updates:** We will provide updates on the vulnerability status within 7 days and keep you informed of our progress.
- **Resolution Timeline:** We aim to address critical vulnerabilities within 30 days, though timing may vary based on complexity.

### Vulnerability Handling Process

1. **Confirmation:** We will confirm the vulnerability and assess its severity.
2. **Fix Development:** A fix will be developed and tested.
3. **Release:** A security update will be released with appropriate versioning.
4. **Disclosure:** After the fix is released, we will disclose the vulnerability with credit to the reporter (unless you prefer to remain anonymous).

### Scope

This security policy applies to:
- The Polykit core library (`polykit-core`)
- The Polykit CLI tool (`polykit`)
- Language adapters (`polykit-adapters`)
- The cache server (`polykit-cache`)

### Out of Scope

The following are considered out of scope for security reporting:
- Denial of service (DoS) attacks that don't compromise data integrity
- Issues requiring physical access to the system
- Issues in dependencies (please report to the upstream project)
- Social engineering attacks

### Security Best Practices

When using Polykit:
- Keep your installation up to date
- Review and validate `polykit.toml` configurations before execution
- Use the `polykit validate` command to check configuration integrity
- Be cautious when using remote cache servers from untrusted sources
- Review task commands before execution, especially in CI/CD pipelines

Thank you for helping keep Polykit secure!
