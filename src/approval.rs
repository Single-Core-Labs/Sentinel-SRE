use crate::domain::{Environment, IncidentInput, MitigationPlan, RiskLevel};

#[derive(Debug, Clone)]
pub struct ApprovalDecision {
    pub approved: bool,
    pub approver: String,
    pub note: String,
}

pub trait ApprovalGate {
    fn request_approval(
        &self,
        incident: &IncidentInput,
        plan: &MitigationPlan,
        reason: &str,
    ) -> ApprovalDecision;
}

#[derive(Debug, Default, Clone)]
pub struct RuleBasedApprovalGate;

impl ApprovalGate for RuleBasedApprovalGate {
    fn request_approval(
        &self,
        incident: &IncidentInput,
        plan: &MitigationPlan,
        reason: &str,
    ) -> ApprovalDecision {
        match (incident.environment, plan.risk_level) {
            (Environment::Prod, RiskLevel::Medium | RiskLevel::High) => ApprovalDecision {
                approved: false,
                approver: "oncall-human".to_string(),
                note: format!("approval denied: {}", reason),
            },
            _ => ApprovalDecision {
                approved: true,
                approver: "oncall-human".to_string(),
                note: format!("approval granted: {}", reason),
            },
        }
    }
}
