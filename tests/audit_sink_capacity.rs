#[cfg(test)]
#[cfg(feature = "control")]
mod tests {
    use std::sync::Arc;
    use ninelives::control::{AuditRecord, AuditSink, MemoryAuditSink, CommandError};

    #[tokio::test]
    async fn memory_audit_sink_enforces_capacity() -> Result<(), CommandError> {
        let capacity = 100;
        let sink = Arc::new(MemoryAuditSink::new(capacity));

        // Add 150 entries
        for i in 0..150 {
            sink.record(AuditRecord {
                id: format!("req-{}", i),
                label: format!("cmd_{}", i),
                principal: "test-user".into(),
                status: "ok".into(),
            }).await?;
        }

        let records = sink.records().await;
        
        // Should have evicted 50 oldest entries
        assert_eq!(records.len(), capacity as usize, "Sink length should match capacity after overflow");

        // Oldest entry should be cmd_50 (cmd_0 through cmd_49 evicted)
        assert_eq!(records.first().unwrap().label, "cmd_50", "Oldest entry should be cmd_50");
        
        // Newest entry should be cmd_149
        assert_eq!(records.last().unwrap().label, "cmd_149", "Newest entry should be cmd_149");
        Ok(())
    }
}
