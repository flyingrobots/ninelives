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

### Built-in Auth Providers (Feature-Gated)

The `ninelives-control` crate provides several built-in `AuthProvider` implementations, each enabled via a Cargo feature flag. Disabling unused features reduces binary size and attack surface.

-   **`trust-quorum` (`--features auth-trust-quorum`)**:
    *   **Description**: Implements a Shiplog/Stargate-style trust quorum model. Authentication requires a configurable threshold of signatures from a trusted roster. Supports chain or attestation-based verification. Can optionally enforce that the entire `CommandEnvelope` is cryptographically signed.
    *   **Dependency**: Reuses the core logic from `stargate-verifier` (crate `stargate_verifier = "0.x"`, interface `stargate_verifier::Verifier`). Expected API contract: `Verifier.verify_signatures(payload_hash, signatures, roster) -> Result<Vec<PublicKey>, Error>`.
    *   **Configuration**: Per-environment configs for:
        *   `roster`: List of trusted public keys/identities.
        *   `threshold`: Minimum number of valid signatures required.
        *   `mode`: `chain` (verify full chain of trust) | `attestation` (verify only trust root).
        *   `require_signed_envelope`: Boolean, if true, the entire envelope must be signed.
        *   `trust_source`: File path or reference to a KMS for the roster.

-   **`jwt` (`--features auth-jwt`)**:
    *   **Description**: Authenticates using OAuth/OIDC JSON Web Tokens (JWTs).
    *   **Dependency**: Leverages the `jsonwebtoken` crate (`jsonwebtoken = "8.x"`).
    *   **Configuration**:
        *   `issuer`: Expected token issuer (e.g., `https://accounts.google.com`).
        *   `audience`: Expected token audience (e.g., `my-service-id`).
        *   `jwks_uri`: URI to fetch JSON Web Key Set for signature verification.
        *   `required_scopes`: List of scopes (e.g., `admin`, `write`) that must be present.
        *   `required_claims`: Map of claim-name to expected-value pairs.

-   **`mtls` (`--features auth-mtls`)**:
    *   **Description**: Authenticates via Mutual TLS (mTLS) client certificates.
    *   **Dependency**: Built using `rustls` for TLS stack (`rustls = "0.21"`, `rustls-pemfile = "1.x"`).
    *   **Configuration**:
        *   `ca_certs_path`: Path to a PEM file containing trusted CA certificates.
        *   `allowed_peer_dns`: List of allowed Subject DNs or SANs from client certificates.
        *   `client_cert_validation_policy`: `strict` (full chain and hostname) | `permissive` (only CA).

-   **`passthrough` (`--features auth-passthrough`)**:
    *   **Description**: A simple provider for testing and development environments. It always authenticates successfully, providing an "anonymous" principal.
    *   **Dependency**: None (built-in).
    *   **Configuration**: None.

Each `AuthProvider` instance registered at runtime is associated with a configured list of allowed `AuthPayload` types it can process. If a provider cannot handle a specific `AuthPayload` (e.g., a `Jwt` provider receiving `AuthPayload::Signatures`), it returns an error and the `AuthRegistry` continues to the next provider based on its `mode`.

### Composition

The `AuthRegistry` orchestrates multiple `AuthProvider` implementations based on a configurable `AuthMode`.

#### **Dispatch Rules**
The `AuthRegistry` iterates through configured `AuthProvider` instances. The choice of providers to try can depend on the `AuthPayload` variant:
- If `AuthPayload::Jwt`, only `Jwt` providers are considered.
- If `AuthPayload::Signatures`, only `Signatures` providers are considered.
- If `AuthPayload::Mtls`, only `mTLS` providers are considered.
- `AuthPayload::Opaque` is passed to all configured providers, allowing custom handling.
- If `AuthPayload` is `None`, only providers configured to handle unauthenticated requests (e.g., `PassthroughAuth` for testing) are considered.

Providers are attempted in the order they are registered with the `AuthRegistry`, unless `AuthMode` dictates otherwise. If no providers match the `AuthPayload` type, the request is denied with an `Unauthenticated` error.

#### **Composition Semantics**
The `AuthRegistry` supports two primary composition modes, configurable per environment:

-   **`mode = "first"` (Fail-Fast / First-Success)**:
    *   **Behavior**: The registry iterates through the selected (dispatch-matching) providers in their configured order. The first provider that successfully authenticates and authorizes the request is used, and its `AuthContext` is returned.
    *   **Failure**: If no provider succeeds, the request is denied. The consolidated error message lists the failure reason from the *last* attempted provider, unless all providers fail with the *same* error, in which case that common error is returned. Otherwise, a generic "no providers succeeded" error is returned, potentially including a summary of distinct failures (controlled by misconfiguration policy).
    *   **Authorization**: The successful provider's `authorize` method is invoked.

-   **`mode = "all"` (Least-Privilege / All-Must-Pass)**:
    *   **Behavior**: All selected (dispatch-matching) providers must successfully authenticate and authorize the request.
    *   **Context Merging**: If multiple providers succeed, their `AuthContext`s are merged.
        *   `principal`: If all principals are identical, that principal is used. If different, an error is returned (misconfiguration).
        *   `provider`: A concatenated string of all successful provider names.
        *   `attributes`: Merged as a union of all attributes. Conflicts are resolved by overwriting (last registered provider wins), or explicitly documented merge rules (e.g., for permissions, take the intersection for least privilege). *Default: union.*
    *   **Authorization Policy**:
        *   Default: All successful providers' `authorize` methods must pass. (Equivalent to `authorize=all`).
        *   Configurable: `authorize=any` (any one successful provider's `authorize` passes).
    *   **Failure**: If any provider fails to authenticate or authorize, the entire request is denied. The error from the first failing provider is propagated.

#### **Misconfiguration Handling**
Defines how the system behaves when authentication/authorization itself is misconfigured (e.g., no providers registered, conflicting merge results).
-   **Default Policy**: Fail-closed. In production environments, misconfiguration always results in denial to prevent unauthorized access.
-   **Configurable**:
    *   `fail_fast_on_misconfig`: `true` (default in prod) -> deny request.
    *   `degrade_gracefully_on_misconfig`: `true` (default in dev/test) -> log warning, attempt to proceed (e.g., with an "anonymous" context), or return a specific error indicating misconfiguration.
-   **Logging**: All misconfiguration events are logged with `WARN` or `ERROR` level, and optionally emitted as telemetry events.

#### **Environment Definition**
An "environment" is a runtime configuration setting, typically specified in a config file (e.g., `ninelives.toml`) or via environment variables, defining active auth providers and their composition mode.
```yaml
# Example Configuration Snippet
auth:
  environment: "production" # or "development", "testing"
  providers:
    - type: "jwt"
      # ... JWT specific config
    - type: "mtls"
      # ... mTLS specific config
  production:
    mode: "all"
    auth_policy: "all" # for mode="all"
    fail_fast_on_misconfig: true
  development:
    mode: "first"
    fail_fast_on_misconfig: false # allow some degradation for dev
    fallback_on_empty_payload: "passthrough" # special rule for dev
```
Compile-time feature flags control *which* built-in providers are compiled into the binary, while runtime configuration determines *which* of those compiled providers are active and how they compose.

#### **Pseudocode for Control Flow**
```pseudocode
function AuthRegistry.authenticate(envelope):
  selected_providers = filter_providers_by_payload_type(envelope.auth, registered_providers)

  if selected_providers is empty and not configured_to_fallback:
    if misconfig_policy is fail_fast:
      log_error("No auth providers for payload type")
      return AuthError::Unauthenticated("no matching providers")
    else:
      log_warning("No auth providers for payload type; falling back to anonymous")
      return AuthContext::anonymous()

  if mode == "first":
    last_error = null
    for provider in selected_providers:
      result = provider.authenticate(envelope.meta, envelope.auth)
      if result is OK:
        if provider.authorize(result.context, envelope.cmd.label, envelope.meta) is OK:
          return result.context
        else:
          last_error = provider.authorize(result.context, envelope.cmd.label, envelope.meta).error // Authz failed
      else:
        last_error = result.error // Auth failed
    
    // All providers failed
    log_error("All auth providers failed", errors=last_error) // Aggregate errors
    return last_error // Propagate last error or consolidated

  else if mode == "all": // (mode == "all")
    successful_contexts = []
    all_auth_errors = []
    
    for provider in selected_providers:
      result = provider.authenticate(envelope.meta, envelope.auth)
      if result is OK:
        if provider.authorize(result.context, envelope.cmd.label, envelope.meta) is OK:
          successful_contexts.add(result.context)
        else:
          all_auth_errors.add(provider.authorize(result.context, envelope.cmd.label, envelope.meta).error)
      else:
        all_auth_errors.add(result.error)

    if all_auth_errors is not empty:
      log_error("Not all auth providers succeeded", errors=all_auth_errors)
      return all_auth_errors.first // Propagate first error or consolidated

    merged_context = merge_auth_contexts(successful_contexts, merge_rules)
    return merged_context
```

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