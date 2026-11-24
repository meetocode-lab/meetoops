# Security Policy

Because MeetoOps manages Docker engines, servers, databases, and interactive terminals, we take security vulnerabilities seriously.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them privately via GitHub Security Advisories (Private Vulnerability Reporting) on the repository, or contact the project maintainers directly with:
- A description of the vulnerability and affected component(s)
- Steps to reproduce or proof of concept
- Potential impact and any suggested remediation

We aim to acknowledge reports within **48 hours** and coordinate a responsible disclosure timeline once a fix is verified.

## Security Architecture

For details on MeetoOps's threat model, RBAC, AEAD encryption, secret masking, and deployment hardening, see:
- [docs/SECURITY.md](./docs/SECURITY.md)
- [docs/SECURITY-DEPLOYMENT.md](./docs/SECURITY-DEPLOYMENT.md)
