use ninelives::telemetry::{emit_best_effort, NullSink, PolicyEvent, RetryEvent};
use std::time::Duration;
use tokio::runtime::Runtime;

#[test]
fn telemetry_overhead_budget_pending() {
    // TODO: implement benchmark-style overhead check for telemetry emission.
    todo!("implement telemetry overhead measurement");
}
