# Security Policy

## Supported versions

`runtime.zero` is pre-alpha. No release is security-supported yet.

## Reporting vulnerabilities

Open a private report through GitHub Security Advisories once enabled for this repository, or contact the maintainer directly through the GitHub profile if advisories are not yet available.

Do not include real secrets, private tokens, OAuth cookies, or sensitive personal data in reports.

## Project boundaries

This project will not intentionally implement malware behavior, stealth persistence, credential theft, evasion, unauthorized account actions, or destructive cleanup without explicit user confirmation and rollback planning.

Security, inventory, update, uninstall, and cleanup modules must preserve the
safety model in `SAFETY.md`. Module execution remains blocked until the
capability, signing, isolation, transaction, revocation, and rollback gates in
`docs/module-trust-and-execution.md` are implemented and reviewed. Production
claims additionally require the equal-platform/equal-module acceptance matrix
in `docs/production-readiness.md`.
