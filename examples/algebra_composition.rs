//! Demonstrating algebraic composition operators in Nine Lives.
//!
//! Shows how to compose resilience strategies using + and | operators.

use ninelives::prelude::*;
use std::time::Duration;
use tower::ServiceBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Nine Lives: Algebraic Composition ===\n");

    // Example 1: Sequential Composition (+)
    println!("1. Sequential Composition: A + B");
    println!("   Policy(outer) + Policy(inner) creates outer(inner(service))\n");

    let timeout1 = TimeoutLayer::new(Duration::from_secs(10))?;
    let timeout2 = TimeoutLayer::new(Duration::from_secs(5))?;

    let _sequential = Policy(timeout1) + Policy(timeout2);
    println!("   Created: Policy(Timeout10s) + Policy(Timeout5s)");
    println!("   Stack: Timeout10s(Timeout5s(Service))\n");

    // Example 2: Fallback Composition (|)
    println!("2. Fallback Composition: A | B");
    println!("   Try primary strategy A, fall back to secondary B on error\n");

    let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
    let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?);

    let _fallback = fast | slow;
    println!("   Created: Policy(Timeout100ms) | Policy(Timeout5s)");
    println!("   Behavior: Try 100ms timeout, fallback to 5s on failure\n");

    // Example 3: Operator Precedence
    println!("3. Operator Precedence: + binds tighter than |");
    println!("   A | B + C is parsed as A | (B + C)\n");

    let a = Policy(TimeoutLayer::new(Duration::from_millis(50))?);
    let b = Policy(TimeoutLayer::new(Duration::from_secs(10))?);
    let c = Policy(TimeoutLayer::new(Duration::from_secs(5))?);

    let _precedence = a | b + c;
    println!("   Created: A | B + C");
    println!("   Parsed as: A | (B + C)");
    println!("   Structure: Timeout50ms | (Timeout10s(Timeout5s(Service)))\n");

    // Example 4: Explicit Parentheses
    println!("4. Explicit Parentheses: (A | B) + C");
    println!("   C wraps the fallback between A and B\n");

    let a = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
    let b = Policy(TimeoutLayer::new(Duration::from_secs(5))?);
    let c = Policy(TimeoutLayer::new(Duration::from_secs(30))?);

    let _explicit = (a | b) + c;
    println!("   Created: (A | B) + C");
    println!("   Structure: Timeout30s(Fallback(Timeout100ms, Timeout5s)(Service))\n");

    // Example 5: Complex Multi-Tier Strategy
    println!("5. Complex Multi-Tier Resilience\n");

    let aggressive = Policy(TimeoutLayer::new(Duration::from_millis(50))?);

    let defensive = Policy(TimeoutLayer::new(Duration::from_secs(10))?)
        + Policy(TimeoutLayer::new(Duration::from_secs(5))?);

    let last_resort = Policy(TimeoutLayer::new(Duration::from_secs(60))?);

    let _complex = aggressive | defensive | last_resort;
    println!("   aggressive | defensive | last_resort");
    println!("   Where defensive = Policy(T10s) + Policy(T5s)");
    println!();
    println!("   Behavior:");
    println!("   1. Try aggressive (50ms timeout)");
    println!("   2. On failure, try defensive (10s + 5s timeouts)");
    println!("   3. On failure, try last resort (60s timeout)");

    // Actually use one in a ServiceBuilder to show it works
    println!("\n6. Using in ServiceBuilder\n");

    let policy = Policy(TimeoutLayer::new(Duration::from_secs(5))?)
        + Policy(TimeoutLayer::new(Duration::from_secs(1))?);

    let _svc = ServiceBuilder::new()
        .layer(policy)
        .service_fn(|req: &'static str| async move {
            Ok::<_, std::io::Error>(format!("Response: {}", req))
        });

    println!("   ✓ ServiceBuilder::new().layer(policy).service_fn(...)");
    println!("   Ready to handle requests with composed resilience!\n");

    Ok(())
}
