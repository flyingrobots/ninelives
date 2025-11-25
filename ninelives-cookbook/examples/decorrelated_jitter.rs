//! Decorrelated jitter demo.
use ninelives::prelude::*;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let jitter = Jitter::decorrelated(Duration::from_millis(50), Duration::from_secs(2))?;
    for _ in 0..5 {
        let d = jitter.apply_stateful();
        println!("decorrelated sleep: {:?}", d);
    }
    Ok(())
}
