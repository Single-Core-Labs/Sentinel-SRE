use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Mock,
    Real,
}

impl RuntimeMode {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "real" => RuntimeMode::Real,
            _ => RuntimeMode::Mock,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub prometheus_url: String,
    pub loki_url: String,
    pub k8s_api_url: String,
    pub k8s_namespace: String,
    pub service_label_key: String,
    pub bearer_token: Option<String>,
    pub remediation_webhook_url: Option<String>,
}

impl ProviderConfig {
    pub fn from_env() -> Self {
        Self {
            prometheus_url: env::var("SC_PROMETHEUS_URL")
                .unwrap_or_else(|_| "http://localhost:9090".to_string()),
            loki_url: env::var("SC_LOKI_URL").unwrap_or_else(|_| "http://localhost:3100".to_string()),
            k8s_api_url: env::var("SC_K8S_API_URL")
                .unwrap_or_else(|_| "https://kubernetes.default.svc".to_string()),
            k8s_namespace: env::var("SC_K8S_NAMESPACE").unwrap_or_else(|_| "default".to_string()),
            service_label_key: env::var("SC_SERVICE_LABEL_KEY")
                .unwrap_or_else(|_| "app".to_string()),
            bearer_token: env::var("SC_API_BEARER_TOKEN").ok(),
            remediation_webhook_url: env::var("SC_REMEDIATION_WEBHOOK_URL").ok(),
        }
    }
}
