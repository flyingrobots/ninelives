# Security Policy

## Reporting a Vulnerability

- **Email:** security@flyingrobots.dev
- **Report Requirements:** Reports must include a minimal, reproducible example (steps or code) and the exact affected version(s)/range.
- **Acknowledgement and Triage:**
    * We will **acknowledge** receipt of the report within **72 hours**.
    * A **mitigation plan** (triage summary + next steps) will be provided with timelines beginning upon **initial report receipt** and running concurrently with the 72-hour acknowledgement window.
- **Remediation Timelines (from initial report receipt):**
    * **Critical:** Mitigation plan within **7 days**; Target remediation within **7 days**.
    * **High/Medium:** Mitigation plan within **14 days**; Target remediation within **30 days** (High) or **90 days** (Medium).
    * **Low:** Mitigation plan within **30 days**; Target remediation within **180 days** (or faster when feasible).

---

## 🤝 Disclosure & Coordination

- **Embargo:** Issues remain private for **90 days** from the initial report or until a public fix is released, whichever is sooner.
- **Pre-release Fix Evaluation:** If a pre-release fix is provided to you within the 7-day critical/mitigation window, a **30-day private evaluation period** starts on the delivery date. **Additional pre-release fixes do not extend or restart** this 30-day period. The private evaluation ends earlier if a public release is published before the 30 days elapse.
- **Good-Faith Pledge:** Researchers following this process and avoiding service disruption will not face punitive action.
- **Scope:**
    * **In-scope** includes this repository’s source, published crates, shipped configs, and **security-related supply-chain vulnerabilities, including both direct and transitive dependencies**, that can cause runtime failures, degraded CI, or other operational security impacts (e.g., a vulnerable dependency that enables RCE).
    * **Out-of-scope** are purely operational/non-security incidents (e.g., CI runner outages) and non-security bugs.
- **CVE:** We will request and coordinate CVE IDs before or upon public patch release (or per an agreed coordinated disclosure timeline).
- **Verification:** A fix is complete when a regression test or reproduction passes and manual validation confirms resolution.
- **Notifications:** Advisories will be published via GitHub Security Advisories and CHANGELOG within 24 hours of release.

---

## 🗑️ Data Handling and PII

- **PII Sanitation:** **Security reports must be sanitized before submission** so they never contain Personally Identifiable Information (PII).
- **Data Retention:** Sanitized (non-PII) report data and analysis are retained for **365 days** after resolution and then deleted.
- **PII Error Handling:** If PII is discovered in a report due to an error, it must be **immediately redacted and escalated to maintainers**. Any retained PII must be **deleted within 180 days** of resolution.
    * **PII Exception:** Any exception to the 180-day deletion rule requires written maintainer approval with a documented justification and expiry date. **Upon expiry of any exception, the designated maintainer or data owner must delete the retained PII within 5 business days and store a documented confirmation of deletion (timestamp, responsible person, and method) with the incident record.**

---

## 🔄 Supported Versions and Backports

- **Supported Versions:** The prior major/minor release remains supported for **30 days** after the subsequent release; **only the most recent prior release is supported at any given time**.
- **Backports:** Critical/High fixes are backported to the previous supported release when feasible. Advisories will state the first fixed version.

### Upgrade Urgently If Stuck on an Older Version

- If a vulnerability is fixed in v1.0.1 and you cannot upgrade within the support window, you may be exposed. Contact security@flyingrobots.dev with dependency constraints, risk assessment, and proposed mitigations to request an exception. **Exception requests will receive a response within 14 business days.**
