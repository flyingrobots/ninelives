//! Minimal retry-only example.
use ninelives::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ResilienceError<std::io::Error>> {
    let policy = RetryPolicy::builder()
        .max_attempts(3)
        .backoff(
            Backoff::exponential(Duration::from_millis(200))
                .with_max(Duration::from_secs(2))
                .expect("valid backoff cap"),
        )
        .with_jitter(Jitter::full())
        .build()
        .expect("valid retry policy");

    let value = policy
        .execute(|| async {
            // Replace with your real fallible work
            Ok::<_, ResilienceError<std::io::Error>>("hello from retry")
        })
        .await?;

    println!("{}", value);
    Ok(())
}
