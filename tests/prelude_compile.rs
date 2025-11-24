use ninelives::prelude::*;
use std::time::Duration;

#[test]
fn prelude_reexports_core_types() {
    let _backoff = Backoff::constant(Duration::from_millis(1));
    let _jitter = Jitter::None;
    let _retry = RetryPolicy::<std::io::Error>::builder()
        .backoff(_backoff.clone())
        .with_jitter(_jitter.clone())
        .build();
    let _timeout = TimeoutPolicy::new(Duration::from_millis(10));
    let _stack: ResilienceStack<std::io::Error> = ResilienceStack::default();
    assert!(_timeout.is_ok());
}
