# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |

## Reporting a Vulnerability

Please report vulnerabilities to **<james@flyingrobots.dev>**. We aim to acknowledge reports within 48 hours.

## ⚠️ Production Security Warning

### Control Plane Authentication

The default `PassthroughAuth` provider is for **development and testing only**. It effectively disables authentication.

**DO NOT** deploy `PassthroughAuth` to a production environment exposed to untrusted networks. You MUST configure a secure `AuthProvider` (e.g., verify JWTs or mTLS certificates) before exposing the Control Plane.

When using `AuthMode::All`, all providers must succeed; the first successful provider’s principal is kept and later providers may overwrite/merge attributes. Use distinct providers intentionally and prefer `AuthMode::First` unless you need merged attributes.

### Transport Security

The core library provides transport abstractions but does not enforce encryption on the wire by default (unless using a transport adapter that specifically provides it, like HTTPS/TLS). Ensure your Control Plane transport is secured via TLS (e.g., using `ninelives-etcd` with TLS enabled or running behind a secure proxy/mesh) to prevent command injection or eavesdropping.

### Logging & Telemetry Hygiene

- **Do not log** raw JWTs/bearer tokens, mTLS peer certificates/chains, private keys, opaque auth payloads, or secret-bearing config values.
- Use structured logging with redaction/masking or hashing for sensitive fields before they leave the process.
- Audit telemetry/metrics sinks (and CI artifacts) to ensure they never export auth tokens or confidential configuration; apply automated redaction/secret-scanning (e.g., git-secrets, truffleHog, GitHub secret scanning) to catch leaks.
- Apply redaction filters in custom sinks and dashboards; align practices with `docs/ADR-002-auth.md`.
