# TODO

## Backoff improvements
- [ ] Expand `src/backoff.rs` module docs with detailed description, usage example (exponential with `with_max`), and overflow behavior notes (saturate to a documented cap; attempts beyond `u32::MAX` are clamped).
- [ ] Derive `PartialEq` and `Eq` for `Backoff` to enable direct comparisons in tests.
- [ ] Introduce a structured `BackoffError` (implements `Display`/`Error`) and update `with_max` to return `Result<Self, BackoffError>` with clear variants (e.g., `ConstantDoesNotSupportMax`, `MaxMustBePositive`, `MaxLessThanBase`).
- [ ] Validate `with_max`: reject `Duration::ZERO`, and ensure `max >= base` for linear/exponential variants; fix grammar in the error message.
- [ ] Normalize attempt semantics across strategies (decide contract: attempt 0 = no delay vs base delay) and adjust exponential math accordingly; document and update tests.
- [ ] Replace overflow fallbacks with a sane cap (e.g., 1 hour/day) instead of `Duration::from_secs(u64::MAX)`; apply consistently to linear/exponential.
- [ ] Add/adjust tests: linear with max cap progression, base > max validation, zero-base behavior, very large attempt clamping, exponential sequence expectations, overflow saturation assertion.

## Jitter follow-ups
- [ ] Replace deprecated `gen_range` calls with `random_range` (rand 0.9) to remove warnings.

## Workflow/doc hygiene
- [ ] Re-check doctests after backoff/jitter doc changes to ensure examples compile.
