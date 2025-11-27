#![allow(missing_docs)]

use ninelives::BulkheadPolicy;
use ninelives::ResilienceError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn bulkhead_grows_when_adaptive_increases() {
    let policy = BulkheadPolicy::new(1).unwrap();
    let handle = policy.adaptive_max_concurrent();

    let started = Arc::new(AtomicUsize::new(0));
    let notify = Arc::new(tokio::sync::Notify::new());

    // First task holds permit
    let bh = policy.clone();
    let started1 = started.clone();
    let notify1 = notify.clone();
    let holder = tokio::spawn(async move {
        bh.execute(|| {
            let s = started1.clone();
            let n = notify1.clone();
            async move {
                s.fetch_add(1, Ordering::SeqCst);
                n.notify_one();
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok::<_, ResilienceError<std::io::Error>>(())
            }
        })
        .await
    });

    notify.notified().await;

    // Second task should be rejected with capacity=1
    let res = policy.execute(|| async { Ok::<_, ResilienceError<std::io::Error>>(()) }).await;
    assert!(matches!(res, Err(e) if e.is_bulkhead()));

    // Increase capacity live
    handle.set(2);

    // Third task should now succeed
    let res = policy.execute(|| async { Ok::<_, ResilienceError<std::io::Error>>(()) }).await;
    assert!(res.is_ok());

    let _ = holder.await;
}
