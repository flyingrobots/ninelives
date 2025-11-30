# ADR-001: Repo & Ecosystem Split Plan

## Status

Proposed

## Context

We want a lean core crate (`ninelives`) and a set of optional plugins (sinks, backends, transports). Users should opt in to heavy dependencies; contributors need clarity on where code lives. Feature flags per crate are fine, but we prefer isolating heavy deps in separate crates to keep core fast and stable.

## Decision

- Keep current repo as **core** for now (core crate + basic sinks + cookbook).
- Target future split into two repos:
  1) `ninelives-core` repo: core crate only (no heavy deps, minimal examples).
  2) `ninelives-ecosystem` repo: workspace of optional plugins (sinks, backends, transports). Optionally add an umbrella crate with feature flags re-exporting individual plugins.
- Keep an empty default feature set for the umbrella to avoid pulling heavy deps by default.
- Maintain per-crate READMEs and tests; CI matrices run per crate with `--all-features` for plugins, lightweight lane for core.

## Rationale

- Core stays tiny and fast to build/test; blast radius is small.
- Plugins can evolve and release independently but still share one ecosystem repo for cross-plugin changes and shared CI.
- Users choose individual crates or the umbrella crate with feature flags.
- Avoids repo sprawl of one-repo-per-plugin while still isolating heavy deps from core.

## Consequences

- Until the split happens, we keep the current workspace. When we split, update `Cargo.toml` paths to versions, adjust README links, and create CI in the ecosystem repo.
- Publishing flow: publish core first, then plugins; tag once per release wave if versions are aligned.

## Open Questions

- When to execute the split? Trigger: first stable release or when plugin churn starts slowing core CI.
- Do we also extract cookbook into its own crate/repo? Leaning yes if examples grow heavy; otherwise keep in core.
