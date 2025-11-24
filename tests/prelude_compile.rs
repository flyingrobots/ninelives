//! Compile-time prelude coverage test (algebra-focused).
use ninelives::prelude::*;
use std::time::Duration;
use tower::service_fn;
use tower_layer::Layer;
use tower_service::Service;

#[tokio::test]
async fn prelude_reexports_core_types() {
    let _backoff = Backoff::constant(Duration::from_millis(1));
    let _jitter = Jitter::None;
    let timeout_layer = TimeoutLayer::new(Duration::from_millis(10)).unwrap();
    let composed = Policy(timeout_layer);

    let mut svc = composed.layer(service_fn(|_req: ()| async { Ok::<_, std::io::Error>(()) }));
    let _ = svc.call(()).await.unwrap();
}
