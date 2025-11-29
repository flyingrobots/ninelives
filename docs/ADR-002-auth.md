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

### Transport Integration: Canonical Mappings and Fallback

Transports are responsible for extracting raw authentication credentials from the incoming channel (e.g., HTTP headers, gRPC metadata, JSONL fields) and mapping them to a canonical `AuthPayload` variant within the `CommandEnvelope`. Transports **do not** perform verification; they merely populate the `AuthPayload`. The `AuthRegistry` expects a consistent `AuthPayload` format regardless of the originating transport.

#### **1. HTTP/REST Transport**

*   **JWT**:
    *   **Source**: `Authorization: Bearer <token>` header.
    *   **Mapping**: `<token>` maps to `AuthPayload::Jwt { token: <token> }`.
    *   **Precedence**: If multiple `Authorization: Bearer` headers are present, the first one encountered (or a configurable transport-specific policy) is used.
*   **Basic Auth**:
    *   **Source**: `Authorization: Basic <credentials>` header.
    *   **Mapping**: The base64-decoded `<credentials>` (e.g., `user:password`) are mapped to `AuthPayload::Opaque(<Vec<u8> of user:password>)`. A dedicated BasicAuth provider (if enabled via `auth-basic-auth` feature) is responsible for parsing this opaque data. Direct mapping to JWT is *not* supported; Basic Auth is primarily for legacy clients and should be explicitly handled.
*   **Multiple Creds**: If both `Bearer` and `Basic` headers are present, `Bearer` takes precedence.

#### **2. gRPC Transport**

*   **JWT**:
    *   **Source**: `authorization` metadata key with value `Bearer <token>`.
    *   **Mapping**: `<token>` maps to `AuthPayload::Jwt { token: <token> }`.
*   **mTLS**:
    *   **Source**: Peer certificate presented during TLS handshake.
    *   **Mapping**: The transport extracts the peer certificate chain and its Distinguished Name (DN) from the TLS context. Maps to `AuthPayload::Mtls { peer_dn: <peer_dn>, cert_chain: <Vec<Vec<u8>> of DER-encoded certs> }`. This requires `tonic::transport::Server` to be configured with mTLS and client certificate negotiation.

#### **3. JSONL (Stdin/File) Transport**

*   **Schema**: JSON objects read from input are expected to contain a top-level `auth` field.
*   **Example**:
    ```json
    {
      "id": "cmd-456",
      "cmd": "get_state",
      "args": {},
      "auth": {
        "type": "signatures",
        "payload_hash": "sha256:...",
        "signatures": [
          {"algorithm": "ed25519", "signature": "...", "key_id": "..."}
        ]
      }
    }
    ```
*   **Mapping**: The `auth` field is directly deserialized into the `AuthPayload` enum. The `type` field in JSON (e.g., `"signatures"`, `"opaque"`, `"jwt"`, `"mtls"`) maps to the corresponding `AuthPayload` variant.
*   **Parsing Rules**: Strict JSON parsing is used. Invalid `auth` structures will result in deserialization errors.

#### **4. In-Process Transport**

*   **Mapping**: For direct programmatic access within the same process, it is recommended to pass `AuthPayload::Opaque` containing a native `AuthContext` or an `Identity` struct (e.g., `AuthPayload::Opaque(bincode::serialize(&my_identity)?)`) directly in the `CommandEnvelope`. A dedicated in-process `AuthProvider` (e.g., `PassthroughAuth` or a `LocalIdentityAuth`) can then deserialize and validate this.
*   **Alternative**: In trusted scenarios, the in-process transport could directly construct and inject an already verified `AuthContext` alongside `auth: None` in the `CommandEnvelope::meta`. *Decision: For consistency, all transports should populate `AuthPayload` if possible. Direct `AuthContext` injection is reserved for highly trusted internal bypasses, which should be explicitly documented within `CommandEnvelope::meta` if used.*

#### **Fallback Behavior & Precedence (General)**

*   **Missing Credentials**: If a transport receives no authentication credentials (e.g., no `Authorization` header), it maps to `auth: None` in the `CommandEnvelope`. The `AuthRegistry` will then determine if `AuthPayload::None` is permitted (e.g., by a `PassthroughAuth` provider configured for that purpose).
*   **Multiple Creds (within a single request/transport)**: If a single incoming request contains multiple types of credentials (e.g., both HTTP `Bearer` token and mTLS client cert), the transport should prioritize one over the other based on a predefined or configurable precedence policy (e.g., mTLS > Bearer). If a choice cannot be made or is ambiguous, the transport should typically return an error. The `AuthPayload` enum currently captures only one primary credential type.

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

## Open Questions & Implementation TODOs



This section outlines critical concerns for the implementation RFC, with proposed approaches or explicit TODOs for further refinement.



### **1. Security Concerns**



*   **Timing Attack Mitigation (Constant-Time Comparisons)**:

    *   **Concern**: Sensitive comparisons (e.g., JWT signatures, API keys, password hashes if ever used) could be vulnerable to timing attacks.

    *   **Approach**: All security-critical string/byte comparisons within `AuthProvider` implementations MUST use constant-time comparison functions (e.g., from `subtle` crate or similar).

    *   **TODO**: Ensure `jsonwebtoken` and `stargate-verifier` dependencies use or expose constant-time comparisons.

*   **Credential Refresh/Rotation Policies and TTLs**:

    *   **Concern**: How do `AuthProvider`s handle credential expiration (e.g., JWT expiry) and key rotation (e.g., JWKS updates, trust roster changes)?

    *   **Approach**: `AuthProvider`s will be stateful and periodically refresh their configuration (e.g., JWKS URIs polled at configurable intervals, trust rosters reloaded from disk/KMS). Verification will always use the latest valid key. Expired credentials lead to `AuthError::Unauthenticated`.

    *   **TODO**: Define a standard interface or mechanism for `AuthProvider`s to expose refreshable state, potentially leveraging `ninelives::Adaptive` values for refresh intervals.

*   **Per-Identity Rate Limiting for Failed Authentication Attempts**:

    *   **Concern**: How to prevent brute-force attacks against user/service identities.

    *   **Approach**: The `AuthRegistry` will implement an optional layer of rate limiting on `AuthError::Unauthenticated` responses, keyed by `principal` (if identifiable pre-auth) or source IP. This uses `ninelives::RateLimitLayer`.

    *   **TODO**: Integrate `ninelives::RateLimitLayer` with `AuthRegistry` and make it configurable.



### **2. Operational Concerns**



*   **Secret/KMS Storage and Rotation**:

    *   **Concern**: Where are sensitive configurations (JWKS URLs, CA certs, trust rosters) stored, and how are they rotated securely without service downtime?

    *   **Approach**: `AuthProvider`s will accept paths to files (e.g., PEM files, JSON files) or references to external Key Management Systems (KMS) or configuration services. Changes to these sources should trigger a hot-reload of the `AuthProvider`'s internal state.

    *   **TODO**: Define a standard config interface for specifying secret locations (e.g., `file://`, `s3://`, `vault://`).

*   **Behavior on Cascading Failures (External Dependencies)**:

    *   **Concern**: What happens if an `AuthProvider`'s external dependency (e.g., JWKS endpoint, etcd for trust roster) is unreachable or slow? Does it block the entire authentication flow?

    *   **Approach**: `AuthProvider`s that depend on external services MUST be wrapped in `ninelives` resilience policies (e.g., `TimeoutLayer`, `CircuitBreakerLayer`). A failing external dependency should *only* impact the specific `AuthProvider` and not block the entire `AuthRegistry` (e.g., `mode="first"` should continue to next provider if one times out; `mode="all"` should fail if any provider is unhealthy).

    *   **TODO**: Document how `ninelives` resilience policies can be applied to `AuthProvider` implementations.

*   **Testing/Mocking Interfaces for Providers**:

    *   **Concern**: How to test `AuthProvider`s effectively, especially those with external dependencies, without requiring live infrastructure.

    *   **Approach**: All `AuthProvider` implementations will expose clear, dependency-injectable constructors or builder patterns to allow mocking of external clients/services. Mock contexts/clients will be used in unit/integration tests.

    *   **TODO**: Provide guidelines and examples for mocking `AuthProvider` dependencies.



### **3. Correctness & Semantic Concerns**



*   **Conflict Resolution When Providers Disagree**:

    *   **Concern**: In `mode="all"`, what if two providers authenticate different principals for the same request?

    *   **Approach**: If `principals` differ, the `AuthRegistry` will return an `AuthError::Misconfiguration("conflicting principals")`. For other fields (e.g., `attributes`), merge rules are defined in `Composition Semantics` (union/overwrite).

    *   **TODO**: Explicitly define strictness for principal consistency.

*   **Clear Distinction Between Provider Errors and Auth Failures**:

    *   **Concern**: Is an `AuthError::Internal` (provider bug) distinct from `AuthError::Unauthenticated` (bad credentials)?

    *   **Approach**: `AuthError::Internal` indicates a problem within the provider's logic or an unrecoverable operational issue (e.g., database connection lost). `AuthError::Unauthenticated` means credentials were bad. `AuthError::Unauthorized` means permissions were insufficient. These distinctions must be strictly maintained for accurate logging and metrics.

    *   **TODO**: Review `AuthError` enum to ensure all failure modes are appropriately categorized.

*   **Rules for Provider Side Effects (Logging/Telemetry)**:

    *   **Concern**: When should `AuthProvider`s log, emit telemetry, or have other side effects?

    *   **Approach**: `AuthProvider`s should be designed to be stateless and deterministic for the `authenticate` and `authorize` calls. All side effects (logging, metrics) should be managed by the `AuthRegistry` or an `AuditSink` based on the overall outcome (success/failure) and policy. Individual providers *may* use `tracing` for debug-level internal diagnostics.

    *   **TODO**: Formalize the `AuditSink` integration into the `AuthRegistry` lifecycle for provider-specific auditing.
