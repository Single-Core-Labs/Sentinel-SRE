use crate::domain::{Environment, IncidentInput, MitigationPlan, RiskLevel};

#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub requires_approval: bool,
    pub reason: String,
}

#[derive(Debug, Default, Clone)]
pub struct PolicyEngine;

impl PolicyEngine {
    pub fn evaluate(&self, incident: &IncidentInput, plan: &MitigationPlan) -> PolicyDecision {
        if plan.rollback_steps.is_empty() {
            return PolicyDecision {
                allowed: false,
                requires_approval: false,
                reason: "blocked: mitigation has no rollback steps".to_string(),
            };
        }

        if plan
            .blast_radius_estimate
            .to_ascii_lowercase()
            .contains("all services")
        {
            return PolicyDecision {
                allowed: false,
                requires_approval: false,
                reason: "blocked: blast radius too broad".to_string(),
            };
        }

        match incident.environment {
            Environment::Prod => self.prod_decision(plan),
            Environment::Staging => self.staging_decision(plan),
            Environment::Dev => self.dev_decision(plan),
        }
    }

    fn prod_decision(&self, plan: &MitigationPlan) -> PolicyDecision {
        match plan.risk_level {
            RiskLevel::Low => PolicyDecision {
                allowed: true,
                requires_approval: true,
                reason: "prod low-risk action requires approval".to_string(),
            },
            RiskLevel::Medium => PolicyDecision {
                allowed: true,
                requires_approval: true,
                reason: "prod medium-risk action requires approval".to_string(),
            },
            RiskLevel::High => PolicyDecision {
                allowed: false,
                requires_approval: false,
                reason: "prod high-risk action blocked by policy".to_string(),
            },
        }
    }

    fn staging_decision(&self, plan: &MitigationPlan) -> PolicyDecision {
        match plan.risk_level {
            RiskLevel::Low => PolicyDecision {
                allowed: true,
                requires_approval: false,
                reason: "staging low-risk action allowed".to_string(),
            },
            RiskLevel::Medium | RiskLevel::High => PolicyDecision {
                allowed: true,
                requires_approval: true,
                reason: "staging medium/high risk requires approval".to_string(),
            },
        }
    }

    fn dev_decision(&self, plan: &MitigationPlan) -> PolicyDecision {
        match plan.risk_level {
            RiskLevel::High => PolicyDecision {
                allowed: true,
                requires_approval: true,
                reason: "dev high-risk action requires approval".to_string(),
            },
            RiskLevel::Low | RiskLevel::Medium => PolicyDecision {
                allowed: true,
                requires_approval: false,
                reason: "dev low/medium risk action allowed".to_string(),
            },
        }
    }
}
