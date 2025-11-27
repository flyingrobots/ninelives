use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Plan {
    pub summary: &'static str,
    pub steps: &'static [&'static str],
    pub ready: &'static [&'static str],
    pub unit: &'static [&'static str],
    pub integ: &'static [&'static str],
    pub value: Option<&'static str>,
}

pub fn p2_plans() -> HashMap<&'static str, Plan> {
    use Plan as P;
    let mut m = HashMap::new();
    m.insert(
        "P2.06",
        P {
            summary: "Bulkhead max_concurrent is backed by DynamicConfig so live updates change capacity for new acquires.",
            steps: &["Add unit tests for permit growth/shrink handling", "Implement/verify dynamic capacity update in bulkhead", "Document live tuning notes and shrink semantics"],
            ready: &["Unit tests cover growth and shrink behavior", "Docs describe tuning and shrink limitations"],
            unit: &["Increase max_concurrent at runtime adds permits for subsequent requests", "Decreasing max_concurrent does not deadlock; either no-op or documented behavior", "Concurrent acquire under update does not panic"],
            integ: &["Control-plane write updates max_concurrent and is respected on next calls"],
            value: Some("M"),
        },
    );
    m.insert(
        "P2.07",
        P {
            summary: "Define CommandContext schema (id, args, identity, optional response channel placeholder).",
            steps: &["Specify struct and serde derive", "Validate required fields", "Doc schema and examples"],
            ready: &["Serde roundtrips succeed", "Docs show sample context payload"],
            unit: &["Serde roundtrip preserves id/args/identity", "Missing required fields errors as expected"],
            integ: &["Router accepts CommandContext constructed via serde_json from sample payload"],
            value: Some("H"),
        },
    );
    m.insert(
        "P2.08",
        P {
            summary: "Define CommandHandler trait as tower::Service<CommandContext>.",
            steps: &[
                "Create trait alias/type",
                "Add blanket impl sanity test",
                "Doc contract (poll_ready/call)",
            ],
            ready: &[
                "Trait compiles with tower Service bounds",
                "Docs describe poll_ready/call expectations",
            ],
            unit: &[
                "Compile-time test using a dummy Service implementing CommandHandler",
                "ServiceBuilder with CommandHandler trait object type-checks",
            ],
            integ: &["Router can be built with a mock CommandHandler service"],
            value: Some("M"),
        },
    );
    m.insert(
        "P2.09",
        P {
            summary:
                "Implement ControlPlaneRouter skeleton: auth, dispatch to handler, history append.",
            steps: &[
                "Add auth pass/fail coverage",
                "Dispatch to handler and record history",
                "Handle errors consistently",
            ],
            ready: &["Auth success/failure tests pass", "History records executed commands"],
            unit: &[
                "Auth failure short-circuits handler",
                "Successful command calls handler and appends history",
                "Handler error propagates as CommandError::Handler",
            ],
            integ: &["End-to-end with PassthroughAuth + mock handler records meta in history"],
            value: Some("H"),
        },
    );
    m.insert(
        "P2.10",
        P {
            summary: "Parameter handlers to set/get adaptive values via control plane.",
            steps: &[
                "Add tests for write/read flows",
                "Implement set/get with validation",
                "Document supported paths",
            ],
            ready: &[
                "Set/Get update adaptive handles and surface errors",
                "Docs list config paths",
            ],
            unit: &[
                "WriteConfig updates adaptive and returns Ack",
                "ReadConfig returns current value",
                "Unknown path returns error",
            ],
            integ: &["Router executes WriteConfig then ReadConfig reflecting new value"],
            value: Some("H"),
        },
    );
    m.insert(
        "P2.11",
        P {
            summary: "State handler to query policy state (placeholder until Observer).",
            steps: &[
                "Define response schema",
                "Return stub/state from in-memory source",
                "Doc limitations",
            ],
            ready: &[
                "Handler compiles and returns structured state",
                "Docs note placeholder nature",
            ],
            unit: &["Handler returns NotImplemented or stub state without panic"],
            integ: &["Router dispatches GetState and surfaces response"],
            value: Some("M"),
        },
    );
    m.insert(
        "P2.12",
        P {
            summary: "ResetCircuitBreaker handler to trip reset/open/half-open appropriately.",
            steps: &["Define command payload", "Wire to breaker handle", "Doc safety/visibility"],
            ready: &["Reset command changes breaker state", "Docs describe effect"],
            unit: &[
                "Reset command transitions breaker to closed and clears counts",
                "Reset when already closed is idempotent",
            ],
            integ: &["Router invokes ResetCircuitBreaker and state handler reflects closed"],
            value: Some("M"),
        },
    );
    m.insert(
        "P2.13",
        P {
            summary: "ListPolicies handler enumerates registered policies/services.",
            steps: &["Track registry of policy IDs", "Return stable list", "Doc format"],
            ready: &["Handler returns non-empty when registered", "Docs specify ID format"],
            unit: &[
                "List returns registered IDs in deterministic order",
                "Empty registry returns empty list",
            ],
            integ: &["Router returns list after registering mock policies"],
            value: Some("L"),
        },
    );
    m.insert(
        "P2.14",
        P {
            summary: "Transport abstraction design for HTTP/gRPC friendliness (types + traits).",
            steps: &[
                "Define transport trait/s (request/response mapping)",
                "Document wire formats",
                "Prepare for adapters",
            ],
            ready: &["Design doc/traits checked in", "Examples show mapping to HTTP JSON"],
            unit: &["Trait compiles with router types", "Serde models for payload compile"],
            integ: &["Mock transport adapter compiles against trait"],
            value: Some("H"),
        },
    );
    m.insert(
        "P2.15",
        P {
            summary: "In-process transport using channels for local control-plane calls.",
            steps: &[
                "Channel request/response structs",
                "Spawn worker handling router.execute",
                "Doc usage",
            ],
            ready: &["Requests roundtrip through channel transport", "Docs include example"],
            unit: &[
                "Send command through channel and receive CommandResult",
                "Shutdown/close does not panic",
            ],
            integ: &["Use channel transport to update adaptive and observe effect"],
            value: Some("M"),
        },
    );
    m.insert(
        "P2.16",
        P {
            summary: "Authorization layer to enforce identity-based access on commands.",
            steps: &[
                "Define authz policy interface",
                "Add deny/allow tests",
                "Document how to plug providers",
            ],
            ready: &["AuthZ denies unauthorized commands", "Docs list required identity fields"],
            unit: &[
                "Unauthorized command returns AuthError::Unauthorized",
                "Authorized passes through",
            ],
            integ: &["Router wrapped with AuthZ layer blocks disallowed command"],
            value: Some("H"),
        },
    );
    m.insert(
        "P2.17",
        P {
            summary: "Audit layer to log/record all commands for observability.",
            steps: &[
                "Define audit sink interface",
                "Add log capture tests",
                "Doc redaction guidance",
            ],
            ready: &["Audit records command metadata", "Docs note PII/redaction"],
            unit: &["Audit layer logs command id/label", "Errors still emit audit record"],
            integ: &["Audit layer + router emits record when command executed"],
            value: Some("M"),
        },
    );
    m.insert(
        "P2.18",
        P {
            summary: "Wrap router with AuthZ + Audit policies, ensure ordering and error handling.",
            steps: &[
                "Compose layers in correct order",
                "Test pass/deny paths",
                "Doc layering guidance",
            ],
            ready: &["Wrapped router enforces authz then audit", "Docs show composition snippet"],
            unit: &[
                "AuthZ deny short-circuits audit? (document decision)",
                "Successful command audited",
            ],
            integ: &["End-to-end wrapper with passthrough authz and audit sink"],
            value: Some("H"),
        },
    );
    m.insert(
        "P2.19",
        P {
            summary: "Package ninelives-control crate: lib layout, features, reexports.",
            steps: &["Create crate structure", "Expose public API surface", "Doc Cargo features"],
            ready: &["Crate builds and exports router/handlers", "Docs list features"],
            unit: &["crate::prelude reexports compile in doctest"],
            integ: &["cargo test -p ninelives-control succeeds"],
            value: Some("H"),
        },
    );
    m.insert(
        "P2.20",
        P {
            summary: "Docs + examples for control plane (cookbook entries).",
            steps: &["Write README", "Add example usage", "Link from main docs"],
            ready: &["README with quickstart present", "Example runs (cargo run --example)"],
            unit: &[],
            integ: &["Examples compile and run under cargo test --examples"],
            value: Some("M"),
        },
    );
    m
}
