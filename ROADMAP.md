# SClaw Roadmap
## v0.1 (current baseline)
- Rust core orchestration loop with state machine
- Policy + approval gating
- Mock adapter mode for local development
- Initial real adapter scaffold (Prometheus/Loki/Kubernetes + remediation webhook)

## v0.2
- Typed provider clients per backend with richer parsing
- Retry/backoff + circuit-breaking for adapter calls
- Scenario replay harness for historical incident evaluation
- Configurable confidence thresholds for mitigation eligibility

## v0.3
- Plugin system for custom tool adapters
- Multi-service dependency graph awareness
- Postmortem artifact generation from audit timeline
- Optional chat interface (Slack/CLI integration package)

## v1.0 goals
- Stable extension API
- Robust policy packs for staging/prod
- Incident replay scorecards and regression gates in CI
- Production-ready docs and operational runbook
