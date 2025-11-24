# Security Policy

## Reporting a vulnerability

- Email: security@flyingrobots.dev
- Reports must include a minimal, reproducible example (steps or code) and the exact affected version(s)/range.
- We will acknowledge within 72 hours of receipt and provide a fix/mitigation plan within 7 days of the initial report.
- Target fix/mitigation delivery: within 90 days of the initial report for most issues; timelines may accelerate for critical severity.

## Disclosure & coordination

- Embargo: keep issues private until a fix is publicly released, or for 30 days after we provide you with a pre-release fix, whichever occurs sooner.
- Good-faith pledge: researchers following this process and avoiding service disruption will not face punitive action.
- Scope (in): this repository’s source, published crates, and shipped configs; supply-chain issues in our dependencies are welcome.
- Scope (out): operational issues unrelated to code security (e.g., CI runner availability), and non-security bugs.
- CVE: we will request CVE IDs for fixed vulnerabilities within 7 days of releasing the patch.
- Verification: a fix is complete when a regression test or reproduction passes and manual validation confirms resolution.
- Notifications: advisories will be published via GitHub Security Advisories and CHANGELOG within 24 hours of release.
- Data handling: report details are shared only with maintainers needed to triage/fix; PII is not retained beyond 180 days after resolution.

## Supported versions and backports

- Only the latest release on crates.io is supported; older versions reach EOL immediately upon a new release.
- No backports are provided. Advisories will state the first fixed version so users can upgrade.

### Upgrade urgently if stuck on an older version

- If a vulnerability is fixed in v1.0.1 and you cannot upgrade: you are exposed. Backports are not available; you must upgrade or accept the risk.
- For enterprise/locked-dependency scenarios, consider [strategy: reach out to security contact for guidance on exceptions].
