use ninelives::control::handler::InMemoryConfigRegistry;
use ninelives::adaptive::Adaptive;

#[test]
fn config_registry_read_returns_err_on_poison() {
    let reg = InMemoryConfigRegistry::new();
    reg.register("k", Adaptive::new(1u32), |s| s.parse().map_err(|_| "parse".into()), |v| v.to_string());
    // Manually poison the lock by causing a panic while holding it
    let entries = &reg.entries;
    let _ = std::panic::catch_unwind(|| {
        let _guard = entries.write().unwrap();
        panic!("poison");
    });
    let err = reg.read("k").expect_err("should error on poison");
    assert!(err.contains("lock poisoned"));
}
