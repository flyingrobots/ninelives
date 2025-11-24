# Security Policy

## Reporting a vulnerability

- Email: security@flyingrobots.dev
- Reports must include a minimal, reproducible example (steps or code) and the exact affected version(s)/range.
- We will acknowledge within 72 hours of receipt. Mitigation plan (triage summary + next steps) timelines: Critical — within 7 days; High/Medium — within 14 days; Low — within 30 days of the initial report.
- Target remediation timelines: Critical — 7 days; High — 30 days; Medium — 90 days; Low — 180 days from the initial report (or faster when feasible).

## Disclosure & coordination

- Embargo: issues remain private for 90 days from the initial report or until a public fix is released, whichever is sooner. If a pre-release fix is provided to you within the 7-day critical/mitigation window, a 30-day private evaluation period starts on that delivery date, does not restart with additional pre-release fixes, and ends sooner if the public release occurs first.
- Good-faith pledge: researchers following this process and avoiding service disruption will not face punitive action.
- Scope: in-scope includes this repository’s source, published crates, shipped configs, and security-related supply-chain vulnerabilities (including transitive dependencies) that can cause runtime failures, degraded CI, or other operational security impacts—for example, a vulnerable transitive crate that enables RCE in our code paths. Out-of-scope are purely operational/non-security incidents (e.g., CI runner outages) and non-security bugs. If a dependency triggers security-relevant behavior, file it as a security report.
- CVE: we will request and coordinate CVE IDs before or upon public patch release (or per an agreed coordinated disclosure timeline).
- Verification: a fix is complete when a regression test or reproduction passes and manual validation confirms resolution.
- Notifications: advisories will be published via GitHub Security Advisories and CHANGELOG within 24 hours of release.
- Data handling: report details are shared only with maintainers needed to triage/fix. All report data (including non-PII logs and analysis) are retained for 365 days after resolution and then deleted. PII is deleted within 180 days of resolution (defined as the later of public release of the fix or deployment of the required configuration change); any exception requires written maintainer approval with a documented justification and expiry date.

## Supported versions and backports

- Supported versions: the latest release and the prior release on crates.io are supported; the prior release remains supported for 30 days after a new release. Critical/High fixes are backported to the previous supported release when feasible. Advisories will state the first fixed version.

### Upgrade urgently if stuck on an older version

- If a vulnerability is fixed in v1.0.1 and you cannot upgrade within the support window, you may be exposed; contact security@flyingrobots.dev with dependency constraints, risk assessment, and proposed mitigations to request an exception.
