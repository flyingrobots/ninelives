# ADR-002: Pluggable Auth for the Control Plane

## Status
Proposed

## Context
Phase 2 (control plane) needs authentication/authorization that can work with multiple identity models:
- Shiplog/Stargate-style trust quorum (roster + threshold with chain/attestation signatures)
- OAuth/OIDC JWTs
- mTLS or other corporate identity systems
- Local/testing passthrough
We want transports (HTTP/gRPC/JSONL/in-process) to be agnostic; auth should be pluggable and configurable per environment.

## Decision
Introduce a pluggable `AuthProvider` interface and an auth registry. Commands carry an `AuthPayload` in the envelope; transports forward it unchanged. Providers verify and return an `AuthContext`, then perform authorization.

### Types (sketch)
```rust
pub enum AuthPayload {
    Jwt { token: String },
    Signatures { payload_hash: [u8;32], signatures: Vec<DetachedSig> },
    Mtls { peer_dn: String, cert_chain: Vec<Vec<u8>> },
    Opaque(Vec<u8>), // fallback/custom
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
