#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Sev1,
    Sev2,
    Sev3,
    Sev4,
}

impl Severity {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "sev1" => Severity::Sev1,
            "sev2" => Severity::Sev2,
            "sev3" => Severity::Sev3,
            _ => Severity::Sev4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Prod,
    Staging,
    Dev,
}

impl Environment {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "prod" | "production" => Environment::Prod,
            "staging" => Environment::Staging,
            _ => Environment::Dev,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Intake,
    Triage,
    Hypothesis,
    Diagnose,
    MitigationPlan,
    Approval,
    Execute,
    Verify,
    Closeout,
    Closed,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct IncidentInput {
    pub incident_id: String,
    pub service: String,
    pub summary: String,
    pub severity: Severity,
    pub environment: Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Ok,
    Partial,
    Error,
}

#[derive(Debug, Clone)]
pub struct Evidence {
    pub source: String,
    pub detail: String,
    pub value: Option<String>,
    pub confidence: f32,
    pub observed_at: String,
}

#[derive(Debug, Clone)]
pub struct ToolResponse {
    pub status: ToolStatus,
    pub evidence: Vec<Evidence>,
    pub confidence: f32,
    pub safe_next_actions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub title: String,
    pub rationale: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    RestartService,
    ScaleService,
    RollbackRelease,
    FlipFeatureFlag,
}

#[derive(Debug, Clone)]
pub struct MitigationPlan {
    pub action: ActionType,
    pub target: String,
    pub reason: String,
    pub risk_level: RiskLevel,
    pub blast_radius_estimate: String,
    pub rollback_steps: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IncidentReport {
    pub final_state: AgentState,
    pub timeline: Vec<String>,
    pub findings: Vec<String>,
    pub selected_mitigation: Option<MitigationPlan>,
}
