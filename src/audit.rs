use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::AgentState;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: String,
    pub state: AgentState,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, state: AgentState, message: impl Into<String>) {
        self.entries.push(AuditEntry {
            timestamp: unix_seconds_now(),
            state,
            message: message.into(),
        });
    }

    pub fn timeline(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| format!("[{}][{:?}] {}", entry.timestamp, entry.state, entry.message))
            .collect()
    }
}

fn unix_seconds_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
