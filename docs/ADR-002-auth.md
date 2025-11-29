# ADR-002: Pluggable Auth for the Control Plane

Date: 2025-11-25
Owner: James Ross <james@flyingrobots.dev>
Version: 1.0

## Status
Proposed

## Context
Phase 2 (control plane) needs authentication/authorization that can work with multiple identity models:
- [Shiplog](https://github.com/flyingrobots/shiplog)/[Stargate](https://github.com/flyingrobots/git-stargate)-style trust quorum (roster + customizable threshold with chain/attestation signatures)
- OAuth/OIDC JWTs
- mTLS or other corporate identity systems
- Local/testing passthrough
We want transports (HTTP/gRPC/JSONL/in-process) to be agnostic; auth should be pluggable and configurable per environment.

## Decision
Introduce a pluggable `AuthProvider` interface and an auth registry. Commands carry an `AuthPayload` in the envelope; transports forward it unchanged. Providers verify and return an `AuthContext`, then perform authorization.

### Types & Verification Spec
```rust
/// Detached signature identifying the algorithm and public key.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DetachedSig {
    pub algorithm: String,      // "ed25519" or "es256"
    pub signature: Vec<u8>,     // Raw 64-byte (Ed25519) or ASN.1 DER (ECDSA)
    pub key_id: Option<String>, // Optional key identifier
}

pub enum AuthPayload {
    Jwt { token: String },
    Signatures {
        /// SHA-256 hash of the canonical input (transport-specific).
        payload_hash: [u8; 32],
        signatures: Vec<DetachedSig>
    },
    Mtls {
        peer_dn: String,
        /// DER-encoded X.509 certificates, leaf first.
        cert_chain: Vec<Vec<u8>>
    },
    Opaque(Vec<u8>),
}

pub struct CommandEnvelope<C> {
    pub cmd: C,
    pub auth: Option<AuthPayload>,
    pub metadata: CommandMeta,
}

pub trait AuthProvider {
    fn name(&self) -> &'static str;
    fn authenticate(&self, env: &CommandEnvelope<impl CmdTrait>) -> Result<AuthContext, AuthError>;
    fn authorize(&self, ctx: &AuthContext, env: &CommandEnvelope<impl CmdTrait>) -> Result<(), AuthzError>;
}
```

#### Verification Rules

1.  **Payload Hash (`payload_hash`):**
    *   **Algorithm:** SHA-256.
    *   **Input:** The exact raw bytes of the serialized `cmd` payload received by the transport layer.
    *   **Check:** Receiver computes `SHA-256(received_cmd_bytes)` and MUST reject the request if it does not match `payload_hash`.

2.  **Signatures (`DetachedSig`):**
    *   **Verification:** `Verify(public_key, message=payload_hash, signature)`.
    *   **Supported Algorithms:**
        *   `"ed25519"`: Ed25519 signature (64 bytes, raw).
        *   `"es256"`: ECDSA over P-256 with SHA-256 (ASN.1 DER encoded).

3.  **mTLS (`cert_chain`):**
    *   **Format:** List of DER-encoded [X.509](https://tools.ietf.org/html/rfc5280) certificates.
    *   **Order:** `[Leaf, Intermediate(s)..., Root]`.
    *   **Check:** Provider verifies the chain against the configured trust store and validates that `peer_dn` matches the leaf certificate's Subject DN.

### Built-in providers (feature-gated)
- `trust-quorum`: Shiplog/Stargate model (roster + threshold, chain or attestation). Per-environment configs; optional gate requiring the envelope itself be signed. Reuses Stargate-style verifier.
- `jwt`: OIDC/JWT with issuer/audience/JWKS config; optional required scopes/claims.
- `mtls`: Verify peer cert against CA roots + allowed DNs/SANs.
- `passthrough`: testing/dev.

### Composition
- Registry supports `mode = "first"` (first provider that authenticates wins) or `mode = "all"` (all must pass).
- Configurable per environment (dev/test/stage/prod).

### Transport integration
- Transports do not parse credentials; they only populate `AuthPayload` from the incoming channel (HTTP header → Jwt, gRPC metadata → Jwt/Mtls, JSONL stdin → Signatures/Opaque, in-proc → Opaque).
- Command router calls the registry; failures return auth errors; success yields an `AuthContext` for handlers.

### Trust/quorum specifics
- Config: roster (identities), threshold, mode (`chain` | `attestation`), `require_signed` (envelope must be signed), trust source (file/ref). FF-only updates after bootstrap.
- Same logic can front a Stargate-like ingress (local pre-receive) or HTTP/gRPC.

## Rationale
- Keeps core extensible for corporate SSO/OAuth while supporting existing Shiplog/Stargate quorum patterns.
- Transport-agnostic: envelope decouples cred parsing from verification.
- Feature flags avoid pulling heavy deps when not needed.
- Per-env policy lets dev/test be lax and prod strict.

## Consequences
- Adds an auth registry to the control-plane crate; command handlers receive an `AuthContext`.
- Need to define stable `AuthPayload` serialization for JSONL/HTTP/gRPC.
- Must document config examples for JWT and trust-quorum.

## Open Questions
- Do we support "either" JWT OR quorum in one request, or require explicit mode per env? (lean: allow `mode="first"` ordering to cover this.)
- Should we offer built-in mTLS → JWT translation (SPIFFE/SPIRE)?
- How to persist/rotate the trust roster (Git ref vs file vs external KMS)?
