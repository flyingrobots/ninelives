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

### Transport Security

The core library provides transport abstractions but does not enforce encryption on the wire by default (unless using a transport adapter that specifically provides it, like HTTPS/TLS). Ensure your Control Plane transport is secured via TLS (e.g., using `ninelives-etcd` with TLS enabled or running behind a secure proxy/mesh) to prevent command injection or eavesdropping.
