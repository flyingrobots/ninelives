//! Tower-native timeout example with algebraic composition.

use ninelives::prelude::*;
use std::time::Duration;
use tower::{Service, ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Nine Lives: Timeout Example ===\n");

    // Create a service with a 1-second timeout
    let mut svc = ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(1))?)
        .service_fn(|req: &'static str| async move {
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, std::io::Error>(format!("Processed: {}", req))
        });

    // This should succeed (100ms < 1s timeout)
    println!("Calling service (will succeed)...");
    let response = svc.ready().await?.call("fast-request").await?;
    println!("✓ Success: {}\n", response);

    // Now create a service that will timeout
    let mut slow_svc = ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_millis(50))?)
        .service_fn(|_req: &'static str| async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok::<_, std::io::Error>("Should not reach here")
        });

    // This should timeout (1s > 50ms timeout)
    println!("Calling service (will timeout)...");
    match slow_svc.ready().await?.call("slow-request").await {
        Ok(_) => panic!("Expected timeout but request succeeded!"),
        Err(e) => {
            println!("✗ Timeout occurred as expected: {:?}\n", e);
            // In production: match on specific error types for recovery
        }
    }

    // Demonstrate algebraic composition: fallback strategy
    println!("=== Algebraic Composition: Fallback ===\n");

    // Policy is a wrapper that enables algebraic composition operators
    // The | operator creates a fallback: try left, if it fails, try right
    let fast = Policy(TimeoutLayer::new(Duration::from_millis(50))?);
    let slow = Policy(TimeoutLayer::new(Duration::from_secs(2))?);
    let policy = fast | slow; // Try fast first, fallback to slow on timeout
    // Note: The first attempt is cancelled when it times out

    let mut fallback_svc =
        ServiceBuilder::new().layer(policy).service_fn(|req: &'static str| async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<_, std::io::Error>(format!("Processed: {}", req))
        });

    println!("Using fallback policy (fast 50ms | slow 2s)...");
    println!("Request takes 100ms - fast will timeout, slow will succeed");
    let response = fallback_svc.ready().await?.call("request").await?;
    println!("✓ Success via fallback: {}", response);

    println!("Second request takes 100ms - demonstrating repeat fallback");
    let response = fallback_svc.ready().await?.call("request-2").await?;
    println!("✓ Success via fallback: {}", response);

    Ok(())
}
