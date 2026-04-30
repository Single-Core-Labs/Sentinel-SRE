use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::AUTHORIZATION;
use serde_json::{json, Value};

use crate::config::ProviderConfig;
use crate::domain::{
    ActionType, Evidence, Hypothesis, IncidentInput, MitigationPlan, ToolResponse, ToolStatus,
};

pub trait SreTools {
    fn collect_baseline(&self, incident: &IncidentInput) -> Vec<ToolResponse>;
    fn run_diagnostic_checks(&self, incident: &IncidentInput, hypothesis: &Hypothesis) -> ToolResponse;
    fn execute_mitigation(&self, incident: &IncidentInput, plan: &MitigationPlan) -> ToolResponse;
    fn verify_recovery(&self, incident: &IncidentInput) -> ToolResponse;
}

#[derive(Debug, Default, Clone)]
pub struct MockSreTools;

impl SreTools for MockSreTools {
    fn collect_baseline(&self, incident: &IncidentInput) -> Vec<ToolResponse> {
        let summary = incident.summary.to_ascii_lowercase();

        let mut evidence = vec![make_evidence(
            "prometheus",
            "p95 latency",
            Some(if summary.contains("latency") {
                "1320ms"
            } else {
                "410ms"
            }),
            0.92,
        )];

        if summary.contains("error") {
            evidence.push(make_evidence(
                "grafana",
                "5xx error rate",
                Some("8.1%"),
                0.89,
            ));
        }

        if summary.contains("crash") {
            evidence.push(make_evidence(
                "kubernetes",
                "pod restarts",
                Some("14 restarts/10m"),
                0.95,
            ));
        }

        vec![ToolResponse {
            status: ToolStatus::Ok,
            evidence,
            confidence: 0.9,
            safe_next_actions: vec![
                "check recent deployment".to_string(),
                "validate dependency health".to_string(),
            ],
        }]
    }

    fn run_diagnostic_checks(
        &self,
        incident: &IncidentInput,
        hypothesis: &Hypothesis,
    ) -> ToolResponse {
        let summary = incident.summary.to_ascii_lowercase();
        let title = hypothesis.title.to_ascii_lowercase();

        let signal = if title.contains("deployment") || summary.contains("deploy") {
            Some("new deployment started 11m before incident")
        } else if title.contains("capacity") || summary.contains("latency") {
            Some("cpu saturation at 91% on primary pod set")
        } else if title.contains("dependency") || summary.contains("db") {
            Some("db connection pool wait elevated to 220ms")
        } else {
            Some("insufficient direct signal")
        };

        ToolResponse {
            status: ToolStatus::Ok,
            evidence: vec![make_evidence("diagnostics", "hypothesis check", signal, 0.82)],
            confidence: 0.82,
            safe_next_actions: vec!["pick minimal-blast-radius mitigation".to_string()],
        }
    }

    fn execute_mitigation(&self, _incident: &IncidentInput, plan: &MitigationPlan) -> ToolResponse {
        let action = match plan.action {
            ActionType::RestartService => "rollout restart completed",
            ActionType::ScaleService => "scaled replicas from 6 -> 10",
            ActionType::RollbackRelease => "rolled back to previous stable release",
            ActionType::FlipFeatureFlag => "feature flag switched off for impacted cohort",
        };

        ToolResponse {
            status: ToolStatus::Ok,
            evidence: vec![make_evidence("executor", "mitigation result", Some(action), 0.94)],
            confidence: 0.94,
            safe_next_actions: vec!["verify slos over recovery window".to_string()],
        }
    }

    fn verify_recovery(&self, _incident: &IncidentInput) -> ToolResponse {
        ToolResponse {
            status: ToolStatus::Ok,
            evidence: vec![
                make_evidence("prometheus", "p95 latency", Some("280ms"), 0.96),
                make_evidence("grafana", "5xx error rate", Some("0.2%"), 0.95),
            ],
            confidence: 0.95,
            safe_next_actions: vec!["close incident after stability window".to_string()],
        }
    }
}

#[derive(Debug, Clone)]
pub struct RealSreTools {
    client: Client,
    config: ProviderConfig,
}

impl RealSreTools {
    pub fn from_config(config: ProviderConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .map_err(|err| format!("failed to build HTTP client: {}", err))?;
        Ok(Self { client, config })
    }

    fn maybe_auth(&self, req: RequestBuilder) -> RequestBuilder {
        if let Some(token) = &self.config.bearer_token {
            req.header(AUTHORIZATION, format!("Bearer {}", token))
        } else {
            req
        }
    }

    fn get_json(&self, url: &str, query: &[(&str, &str)]) -> Result<Value, String> {
        let request = self.client.get(url).query(query);
        let response = self
            .maybe_auth(request)
            .send()
            .map_err(|err| format!("request failed ({}): {}", url, err))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("non-success status {} from {}", status, url));
        }
        response
            .json::<Value>()
            .map_err(|err| format!("invalid JSON from {}: {}", url, err))
    }

    fn post_json(&self, url: &str, body: Value) -> Result<Value, String> {
        let request = self.client.post(url).json(&body);
        let response = self
            .maybe_auth(request)
            .send()
            .map_err(|err| format!("request failed ({}): {}", url, err))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("non-success status {} from {}", status, url));
        }
        response
            .json::<Value>()
            .or_else(|_| Ok(json!({"status":"accepted"})))
    }

    fn query_prometheus_scalar(&self, query: &str) -> Result<Option<String>, String> {
        let url = format!("{}/api/v1/query", self.config.prometheus_url);
        let payload = self.get_json(&url, &[("query", query)])?;
        let result = payload
            .get("data")
            .and_then(|d| d.get("result"))
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("value"))
            .and_then(|value| value.as_array())
            .and_then(|parts| parts.get(1))
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        Ok(result)
    }

    fn query_loki_top_line(&self, query: &str) -> Result<Option<String>, String> {
        let url = format!("{}/loki/api/v1/query_range", self.config.loki_url);
        let payload = self.get_json(
            &url,
            &[("query", query), ("limit", "1"), ("direction", "backward")],
        )?;
        let line = payload
            .get("data")
            .and_then(|d| d.get("result"))
            .and_then(|r| r.as_array())
            .and_then(|arr| arr.first())
            .and_then(|entry| entry.get("values"))
            .and_then(|v| v.as_array())
            .and_then(|values| values.first())
            .and_then(|sample| sample.as_array())
            .and_then(|parts| parts.get(1))
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        Ok(line)
    }

    fn query_k8s_restart_count(&self, service: &str) -> Result<Option<String>, String> {
        let selector = format!("{}={}", self.config.service_label_key, service);
        let url = format!(
            "{}/api/v1/namespaces/{}/pods",
            self.config.k8s_api_url, self.config.k8s_namespace
        );
        let payload = self.get_json(&url, &[("labelSelector", selector.as_str())])?;
        let mut restarts: i64 = 0;
        let items = payload
            .get("items")
            .and_then(|i| i.as_array())
            .cloned()
            .unwrap_or_default();

        for pod in items {
            if let Some(containers) = pod
                .get("status")
                .and_then(|s| s.get("containerStatuses"))
                .and_then(|c| c.as_array())
            {
                for container in containers {
                    if let Some(count) = container.get("restartCount").and_then(|c| c.as_i64()) {
                        restarts += count;
                    }
                }
            }
        }
        Ok(Some(restarts.to_string()))
    }

    fn emit_remediation(&self, incident: &IncidentInput, plan: &MitigationPlan) -> Result<String, String> {
        if let Some(webhook) = &self.config.remediation_webhook_url {
            let response = self.post_json(
                webhook,
                json!({
                    "incident_id": incident.incident_id.clone(),
                    "service": incident.service.clone(),
                    "action": format!("{:?}", plan.action),
                    "target": plan.target.clone(),
                    "reason": plan.reason.clone(),
                    "risk_level": format!("{:?}", plan.risk_level),
                }),
            )?;
            let status = response
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("accepted");
            Ok(format!("webhook accepted remediation request ({})", status))
        } else {
            Ok("no remediation webhook configured; emitted dry-run only".to_string())
        }
    }
}

impl SreTools for RealSreTools {
    fn collect_baseline(&self, incident: &IncidentInput) -> Vec<ToolResponse> {
        let mut evidence = Vec::new();
        let mut status = ToolStatus::Ok;

        match self.query_prometheus_scalar(
            "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket[5m])) by (le))",
        ) {
            Ok(value) => evidence.push(make_evidence(
                "prometheus",
                "p95 latency",
                value.as_deref(),
                0.9,
            )),
            Err(err) => {
                status = ToolStatus::Partial;
                evidence.push(make_evidence("prometheus", "p95 latency query error", Some(&err), 0.2));
            }
        }

        match self.query_prometheus_scalar("sum(rate(http_requests_total{status=~\"5..\"}[5m]))") {
            Ok(value) => evidence.push(make_evidence(
                "prometheus",
                "5xx error rate",
                value.as_deref(),
                0.88,
            )),
            Err(err) => {
                status = ToolStatus::Partial;
                evidence.push(make_evidence("prometheus", "5xx query error", Some(&err), 0.2));
            }
        }

        let logql = format!("{{service=\"{}\"}} |= \"error\"", incident.service);
        match self.query_loki_top_line(&logql) {
            Ok(value) => evidence.push(make_evidence(
                "loki",
                "latest error log line",
                value.as_deref(),
                0.75,
            )),
            Err(err) => {
                status = ToolStatus::Partial;
                evidence.push(make_evidence("loki", "log query error", Some(&err), 0.2));
            }
        }

        match self.query_k8s_restart_count(&incident.service) {
            Ok(value) => evidence.push(make_evidence(
                "kubernetes",
                "aggregate pod restart count",
                value.as_deref(),
                0.82,
            )),
            Err(err) => {
                status = ToolStatus::Partial;
                evidence.push(make_evidence("kubernetes", "pod query error", Some(&err), 0.2));
            }
        }

        vec![ToolResponse {
            status,
            evidence,
            confidence: if matches!(status, ToolStatus::Ok) { 0.88 } else { 0.66 },
            safe_next_actions: vec![
                "cross-check recent deployment and alert timeline".to_string(),
                "evaluate dependency saturation vs regression".to_string(),
            ],
        }]
    }

    fn run_diagnostic_checks(&self, incident: &IncidentInput, hypothesis: &Hypothesis) -> ToolResponse {
        let mut evidence = Vec::new();
        let mut status = ToolStatus::Ok;
        let h = hypothesis.title.to_ascii_lowercase();

        if h.contains("capacity") {
            match self.query_prometheus_scalar(
                "avg(rate(container_cpu_usage_seconds_total[5m]))",
            ) {
                Ok(value) => evidence.push(make_evidence(
                    "prometheus",
                    "cpu utilization signal",
                    value.as_deref(),
                    0.84,
                )),
                Err(err) => {
                    status = ToolStatus::Partial;
                    evidence.push(make_evidence("prometheus", "cpu query error", Some(&err), 0.2));
                }
            }
        } else if h.contains("deployment") {
            let logql = format!("{{service=\"{}\"}} |= \"deploy\" |~ \"rollout|revision\"", incident.service);
            match self.query_loki_top_line(&logql) {
                Ok(value) => evidence.push(make_evidence(
                    "loki",
                    "deployment trace in logs",
                    value.as_deref(),
                    0.78,
                )),
                Err(err) => {
                    status = ToolStatus::Partial;
                    evidence.push(make_evidence("loki", "deployment log query error", Some(&err), 0.2));
                }
            }
        } else {
            match self.query_prometheus_scalar("sum(rate(process_open_fds[5m]))") {
                Ok(value) => evidence.push(make_evidence(
                    "prometheus",
                    "dependency pressure proxy",
                    value.as_deref(),
                    0.73,
                )),
                Err(err) => {
                    status = ToolStatus::Partial;
                    evidence.push(make_evidence("prometheus", "dependency proxy query error", Some(&err), 0.2));
                }
            }
        }

        ToolResponse {
            status,
            evidence,
            confidence: if matches!(status, ToolStatus::Ok) { 0.82 } else { 0.58 },
            safe_next_actions: vec!["pick smallest blast-radius remediation candidate".to_string()],
        }
    }

    fn execute_mitigation(&self, incident: &IncidentInput, plan: &MitigationPlan) -> ToolResponse {
        match self.emit_remediation(incident, plan) {
            Ok(message) => ToolResponse {
                status: ToolStatus::Ok,
                evidence: vec![make_evidence(
                    "remediation",
                    "execution gateway response",
                    Some(&message),
                    0.88,
                )],
                confidence: 0.88,
                safe_next_actions: vec!["verify SLO recovery over 10-minute window".to_string()],
            },
            Err(err) => ToolResponse {
                status: ToolStatus::Error,
                evidence: vec![make_evidence(
                    "remediation",
                    "execution gateway failure",
                    Some(&err),
                    0.2,
                )],
                confidence: 0.2,
                safe_next_actions: vec!["stop automation and request manual operator action".to_string()],
            },
        }
    }

    fn verify_recovery(&self, _incident: &IncidentInput) -> ToolResponse {
        let latency = self.query_prometheus_scalar(
            "histogram_quantile(0.95, sum(rate(http_request_duration_seconds_bucket[5m])) by (le))",
        );
        let errors = self.query_prometheus_scalar("sum(rate(http_requests_total{status=~\"5..\"}[5m]))");
        let mut evidence = Vec::new();
        let mut status = ToolStatus::Ok;

        match latency {
            Ok(value) => evidence.push(make_evidence(
                "prometheus",
                "post-mitigation p95 latency",
                value.as_deref(),
                0.92,
            )),
            Err(err) => {
                status = ToolStatus::Partial;
                evidence.push(make_evidence("prometheus", "latency verify query error", Some(&err), 0.2));
            }
        }

        match errors {
            Ok(value) => evidence.push(make_evidence(
                "prometheus",
                "post-mitigation 5xx rate",
                value.as_deref(),
                0.92,
            )),
            Err(err) => {
                status = ToolStatus::Partial;
                evidence.push(make_evidence("prometheus", "5xx verify query error", Some(&err), 0.2));
            }
        }

        ToolResponse {
            status,
            evidence,
            confidence: if matches!(status, ToolStatus::Ok) { 0.9 } else { 0.6 },
            safe_next_actions: vec![
                "close incident after stable window".to_string(),
                "attach timeline to postmortem seed".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Toolset {
    Mock(MockSreTools),
    Real(RealSreTools),
}

impl SreTools for Toolset {
    fn collect_baseline(&self, incident: &IncidentInput) -> Vec<ToolResponse> {
        match self {
            Toolset::Mock(inner) => inner.collect_baseline(incident),
            Toolset::Real(inner) => inner.collect_baseline(incident),
        }
    }

    fn run_diagnostic_checks(&self, incident: &IncidentInput, hypothesis: &Hypothesis) -> ToolResponse {
        match self {
            Toolset::Mock(inner) => inner.run_diagnostic_checks(incident, hypothesis),
            Toolset::Real(inner) => inner.run_diagnostic_checks(incident, hypothesis),
        }
    }

    fn execute_mitigation(&self, incident: &IncidentInput, plan: &MitigationPlan) -> ToolResponse {
        match self {
            Toolset::Mock(inner) => inner.execute_mitigation(incident, plan),
            Toolset::Real(inner) => inner.execute_mitigation(incident, plan),
        }
    }

    fn verify_recovery(&self, incident: &IncidentInput) -> ToolResponse {
        match self {
            Toolset::Mock(inner) => inner.verify_recovery(incident),
            Toolset::Real(inner) => inner.verify_recovery(incident),
        }
    }
}
fn make_evidence(source: &str, detail: &str, value: Option<&str>, confidence: f32) -> Evidence {
    Evidence {
        source: source.to_string(),
        detail: detail.to_string(),
        value: value.map(ToString::to_string),
        confidence,
        observed_at: unix_seconds_now(),
    }
}

fn unix_seconds_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}
