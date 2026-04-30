use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct SessionMemory {
    pub checked_sources: Vec<String>,
    pub attempted_hypotheses: Vec<String>,
    pub notes: Vec<String>,
}

impl SessionMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn note_source(&mut self, source: impl Into<String>) {
        self.checked_sources.push(source.into());
    }

    pub fn note_hypothesis(&mut self, hypothesis: impl Into<String>) {
        self.attempted_hypotheses.push(hypothesis.into());
    }

    pub fn note(&mut self, message: impl Into<String>) {
        self.notes.push(message.into());
    }
}

#[derive(Debug, Default)]
pub struct KnowledgeMemory {
    pub runbooks: BTreeMap<String, String>,
    pub owners: BTreeMap<String, String>,
}

impl KnowledgeMemory {
    pub fn with_defaults() -> Self {
        let mut runbooks = BTreeMap::new();
        runbooks.insert(
            "payments-api".to_string(),
            "payments-api-latency-runbook:v3".to_string(),
        );
        runbooks.insert(
            "gateway".to_string(),
            "gateway-error-budget-runbook:v2".to_string(),
        );

        let mut owners = BTreeMap::new();
        owners.insert("payments-api".to_string(), "sre-payments-oncall".to_string());
        owners.insert("gateway".to_string(), "sre-platform-oncall".to_string());

        Self { runbooks, owners }
    }

    pub fn runbook_for(&self, service: &str) -> Option<&String> {
        self.runbooks.get(service)
    }
}
