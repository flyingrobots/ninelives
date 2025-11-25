//! Demonstrates bulkhead concurrency behavior with blocking holders.
use ninelives::{BulkheadPolicy, ResilienceError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bulkhead = BulkheadPolicy::new(1)?;
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_task = counter.clone();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();

    let holder = tokio::spawn({
        let bh = bulkhead.clone();
        async move {
            let counter = counter_for_task;
            let _ = bh
                .execute(|| async {
                    counter.fetch_add(1, Ordering::SeqCst);
                    let _ = started_tx.send(());
                    let _ = release_rx.await;
                    Ok::<_, ResilienceError<std::io::Error>>(())
                })
                .await;
        }
    });

    started_rx.await?;

    let rejected =
        bulkhead.execute(|| async { Ok::<_, ResilienceError<std::io::Error>>(()) }).await;
    assert!(rejected.unwrap_err().is_bulkhead());

    let _ = release_tx.send(());
    let _ = holder.await?;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    Ok(())
}
