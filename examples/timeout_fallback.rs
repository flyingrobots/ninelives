//! Demonstrates combining timeout and fallback policies.
use ninelives::prelude::*;
use std::time::Duration;
use tower::{Service, ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
    let slow = Policy(TimeoutLayer::new(Duration::from_secs(1))?);
    let policy = fast | slow;

    let mut svc = ServiceBuilder::new()
        .layer(policy)
        .service_fn(|req: &'static str| async move {
            if req == "slow" {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Ok::<_, std::io::Error>(req)
        });

    let fast_result = svc.ready().await?.call("ok").await?;
    println!("fast path: {}", fast_result);

    let slow_result = svc.ready().await?.call("slow").await?;
    println!("fallback path: {}", slow_result);

    Ok(())
}
