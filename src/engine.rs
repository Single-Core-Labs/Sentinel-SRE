use crate::approval::ApprovalGate;
use crate::audit::AuditLog;
use crate::domain::{
    ActionType, AgentState, Hypothesis, IncidentInput, IncidentReport, MitigationPlan, RiskLevel,
    ToolResponse, ToolStatus,
};
use crate::memory::{KnowledgeMemory, SessionMemory};
use crate::policy::PolicyEngine;
use crate::tools::SreTools;

pub struct SreAgent<T, A>
where
    T: SreTools,
    A: ApprovalGate,
{
    tools: T,
    policy: PolicyEngine,
    approver: A,
    knowledge: KnowledgeMemory,
}

impl<T, A> SreAgent<T, A>
where
    T: SreTools,
    A: ApprovalGate,
{
    pub fn new(tools: T, policy: PolicyEngine, approver: A, knowledge: KnowledgeMemory) -> Self {
        Self {
            tools,
            policy,
            approver,
            knowledge,
        }
    }

    pub fn run(&self, incident: IncidentInput) -> IncidentReport {
        let mut audit = AuditLog::new();
        let mut memory = SessionMemory::new();
        let mut findings = Vec::new();
        let mut state = AgentState::Intake;

        audit.record(
            state,
            format!(
                "incident {} received for service {}",
                incident.incident_id, incident.service
            ),
        );
        if let Some(runbook) = self.knowledge.runbook_for(&incident.service) {
            audit.record(
                state,
                format!("runbook attached as citation: {}", runbook),
            );
        }

        state = AgentState::Triage;
        audit.record(state, "collecting baseline telemetry");
        let baseline = self.tools.collect_baseline(&incident);
        findings.extend(extract_findings(&baseline));
        for response in &baseline {
            for evidence in &response.evidence {
                memory.note_source(evidence.source.clone());
            }
        }

        state = AgentState::Hypothesis;
        audit.record(state, "generating candidate root-cause hypotheses");
        let hypotheses = build_hypotheses(&incident, &findings);
        for hypothesis in &hypotheses {
            memory.note_hypothesis(hypothesis.title.clone());
        }
        let selected_hypothesis = hypotheses
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .cloned()
            .unwrap_or_else(|| Hypothesis {
                title: "insufficient signal".to_string(),
                rationale: "fallback hypothesis".to_string(),
                confidence: 0.4,
            });
        audit.record(
            state,
            format!(
                "selected hypothesis: {} ({:.2})",
                selected_hypothesis.title, selected_hypothesis.confidence
            ),
        );

        state = AgentState::Diagnose;
        audit.record(state, "running focused diagnostic checks");
        let diagnostic = self
            .tools
            .run_diagnostic_checks(&incident, &selected_hypothesis);
        findings.extend(extract_findings(&[diagnostic.clone()]));
        memory.note(format!(
            "diagnostic confidence: {:.2}",
            diagnostic.confidence
        ));

        state = AgentState::MitigationPlan;
        audit.record(state, "building mitigation plan");
        let mitigation = select_mitigation(&incident, &selected_hypothesis, &diagnostic);
        audit.record(
            state,
            format!(
                "planned action: {:?} targeting {}",
                mitigation.action, mitigation.target
            ),
        );

        state = AgentState::Approval;
        audit.record(state, "evaluating policy for proposed action");
        let policy_decision = self.policy.evaluate(&incident, &mitigation);
        audit.record(
            state,
            format!(
                "policy decision => allowed: {}, requires approval: {}, reason: {}",
                policy_decision.allowed, policy_decision.requires_approval, policy_decision.reason
            ),
        );
        if !policy_decision.allowed {
            state = AgentState::Blocked;
            audit.record(state, "incident blocked by policy");
            return IncidentReport {
                final_state: state,
                timeline: audit.timeline(),
                findings,
                selected_mitigation: Some(mitigation),
            };
        }

        if policy_decision.requires_approval {
            let approval = self.approver.request_approval(
                &incident,
                &mitigation,
                &policy_decision.reason,
            );
            audit.record(
                state,
                format!(
                    "approval result => approved: {}, by: {}, note: {}",
                    approval.approved, approval.approver, approval.note
                ),
            );
            if !approval.approved {
                state = AgentState::Blocked;
                audit.record(state, "incident blocked: approval denied");
                return IncidentReport {
                    final_state: state,
                    timeline: audit.timeline(),
                    findings,
                    selected_mitigation: Some(mitigation),
                };
            }
        }

        state = AgentState::Execute;
        audit.record(state, "executing mitigation");
        let execution = self.tools.execute_mitigation(&incident, &mitigation);
        findings.extend(extract_findings(&[execution]));

        state = AgentState::Verify;
        audit.record(state, "verifying recovery signals");
        let verification = self.tools.verify_recovery(&incident);
        findings.extend(extract_findings(&[verification.clone()]));

        state = AgentState::Closeout;
        audit.record(state, "preparing closeout summary");
        let outcome = if matches!(verification.status, ToolStatus::Ok) {
            "recovery checks healthy"
        } else {
            "recovery uncertain"
        };
        audit.record(state, outcome);

        state = AgentState::Closed;
        audit.record(state, "incident workflow finished");
        IncidentReport {
            final_state: state,
            timeline: audit.timeline(),
            findings,
            selected_mitigation: Some(mitigation),
        }
    }
}

fn extract_findings(responses: &[ToolResponse]) -> Vec<String> {
    let mut results = Vec::new();
    for response in responses {
        for evidence in &response.evidence {
            let value = evidence
                .value
                .as_ref()
                .map_or_else(|| "n/a".to_string(), Clone::clone);
            results.push(format!(
                "{} => {} = {} (conf {:.2})",
                evidence.source, evidence.detail, value, evidence.confidence
            ));
        }
    }
    results
}

fn build_hypotheses(incident: &IncidentInput, findings: &[String]) -> Vec<Hypothesis> {
    let summary = incident.summary.to_ascii_lowercase();
    let has_latency = summary.contains("latency");
    let has_errors = summary.contains("error");
    let deploy_signal = findings
        .iter()
        .any(|entry| entry.to_ascii_lowercase().contains("deployment"));

    let mut hypotheses = Vec::new();
    if has_latency {
        hypotheses.push(Hypothesis {
            title: "capacity saturation".to_string(),
            rationale: "latency pattern suggests compute or connection pool exhaustion".to_string(),
            confidence: 0.78,
        });
    }
    if has_errors || deploy_signal {
        hypotheses.push(Hypothesis {
            title: "deployment regression".to_string(),
            rationale: "errors aligned with rollout window".to_string(),
            confidence: 0.74,
        });
    }
    hypotheses.push(Hypothesis {
        title: "dependency degradation".to_string(),
        rationale: "upstream dependency can amplify latency and errors".to_string(),
        confidence: 0.66,
    });
    hypotheses
}

fn select_mitigation(
    incident: &IncidentInput,
    selected_hypothesis: &Hypothesis,
    diagnostic: &ToolResponse,
) -> MitigationPlan {
    let title = selected_hypothesis.title.to_ascii_lowercase();
    let summary = incident.summary.to_ascii_lowercase();
    let action = if title.contains("capacity") || summary.contains("latency") {
        ActionType::ScaleService
    } else if title.contains("deployment") || summary.contains("deploy") {
        ActionType::RollbackRelease
    } else if summary.contains("crash") {
        ActionType::RestartService
    } else {
        ActionType::FlipFeatureFlag
    };

    let risk_level = match action {
        ActionType::RestartService | ActionType::FlipFeatureFlag => RiskLevel::Low,
        ActionType::ScaleService => RiskLevel::Medium,
        ActionType::RollbackRelease => RiskLevel::Medium,
    };

    MitigationPlan {
        action,
        target: incident.service.clone(),
        reason: format!(
            "{}; diagnostic confidence {:.2}",
            selected_hypothesis.rationale, diagnostic.confidence
        ),
        risk_level,
        blast_radius_estimate: format!("single service: {}", incident.service),
        rollback_steps: vec![
            "capture pre-change SLI snapshot".to_string(),
            "revert action if error rate worsens for 5 minutes".to_string(),
            "page service owner if rollback fails".to_string(),
        ],
    }
}
