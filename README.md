# SClaw
SClaw is an open-source, Rust-based CLI agent for SRE incident response. It helps teams investigate production issues faster using a consistent workflow, evidence collection, policy guardrails, and approval-aware remediation.

## Mission
Build a reliable, developer-friendly incident assistant that makes on-call response faster, safer, and more repeatable without hiding critical decisions from humans.

## Problem statement
SRE teams usually face the same operational pain points:
- Incident triage is slow and manual across multiple tools.
- Different engineers follow different investigation patterns.
- Risky production actions can be executed without consistent checks.
- Incident handoffs and postmortems often miss structured evidence.
- New contributors struggle to test workflows locally without full infrastructure.

SClaw addresses these by providing:
- A single CLI workflow for incident intake, triage, diagnosis, mitigation, and verification.
- Policy and approval checks before risky execution paths.
- Standardized output (text or JSON) for handoffs and automation.
- Mock mode for local development and open-source onboarding.

## Core capabilities
- CLI-first experience with one-shot and interactive modes.
- Evidence-driven state machine:
  `Intake -> Triage -> Hypothesis -> Diagnose -> MitigationPlan -> Approval -> Execute -> Verify -> Closeout`
- Provider adapters for mock and real integrations.
- Policy engine for blast-radius and rollback constraints.
- Approval gate abstraction for human-in-the-loop safety.
- Audit timeline output suitable for incident reports and postmortems.

## Architecture overview
- `src/main.rs`: CLI entrypoint and command execution flow
- `src/cli.rs`: command parsing and runtime options
- `src/engine.rs`: orchestration/state machine logic
- `src/domain.rs`: incident, evidence, mitigation, and state contracts
- `src/tools.rs`: tool abstraction + mock/real adapters
- `src/config.rs`: runtime/provider configuration from environment
- `src/policy.rs`: policy decisions (allow/block/approval required)
- `src/approval.rs`: approval gate interface and default implementation
- `src/audit.rs`, `src/memory.rs`: timeline and runtime memory support

## Installation
### 1) Install Rust toolchain
Choose one:

Windows (PowerShell):
```powershell path=null start=null
winget install -e --id Rustlang.Rustup
```

macOS/Linux:
```bash path=null start=null
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify:
```bash path=null start=null
cargo --version
rustc --version
```

### 2) Clone and build
```bash path=null start=null
git clone <your-repo-url>
cd Sclaw
cargo build
```

### 3) Run tests and checks
```bash path=null start=null
cargo fmt
cargo check
cargo test
```

## Usage
### Command reference
```bash path=null start=null
cargo run -- help
```

Supported top-level commands:
- `run`: execute one incident analysis
- `repl`: interactive assistant-style mode
- `doctor`: validate runtime configuration readiness
- `benefits`: print what value the tool gives teams
- `help`: show command usage

### One-shot incident analysis
```bash path=null start=null
cargo run -- run --service payments-api --summary "latency spike after deploy" --severity sev2 --env staging --mode mock --output text
```

### Interactive mode (assistant-style)
```bash path=null start=null
cargo run -- repl --service payments-api --severity sev2 --env staging --mode mock --output text
```

Inside REPL:
- Type plain text to submit an incident summary.
- Use slash commands:
  - `/help`
  - `/status`
  - `/set service <name>`
  - `/set severity <sev1|sev2|sev3|sev4>`
  - `/set env <prod|staging|dev>`
  - `/set mode <mock|real>`
  - `/set output <text|json>`
  - `/doctor`
  - `/benefits`
  - `/exit`

### JSON output (automation-friendly)
```bash path=null start=null
cargo run -- run --service gateway --summary "5xx spike in prod" --severity sev1 --env prod --mode mock --output json
```

### Configuration diagnostics
```bash path=null start=null
cargo run -- doctor --mode real
```

## Real mode configuration
Real mode reads configuration from environment variables.

Required in most deployments:
- `SC_PROMETHEUS_URL`
- `SC_LOKI_URL`
- `SC_K8S_API_URL`
- `SC_K8S_NAMESPACE`
- `SC_SERVICE_LABEL_KEY`

Optional but commonly needed:
- `SC_API_BEARER_TOKEN`
- `SC_REMEDIATION_WEBHOOK_URL`

PowerShell example:
```powershell path=null start=null
$env:SC_PROMETHEUS_URL="http://localhost:9090"
$env:SC_LOKI_URL="http://localhost:3100"
$env:SC_K8S_API_URL="https://kubernetes.default.svc"
$env:SC_K8S_NAMESPACE="default"
$env:SC_SERVICE_LABEL_KEY="app"
$env:SC_API_BEARER_TOKEN="{{SRE_OBSERVABILITY_TOKEN}}"
$env:SC_REMEDIATION_WEBHOOK_URL="https://example.internal/remediation"

cargo run -- run --service payments-api --summary "error rate spike" --severity sev2 --env prod --mode real --output text
```

If real adapter initialization fails, SClaw falls back to mock mode for continuity.

## How developers use SClaw
1. Start in `mock` mode to validate workflow behavior and contributor changes.
2. Use `repl` during local testing or incident drills.
3. Run `doctor` before enabling `real` mode in a team environment.
4. Integrate JSON output with scripts, CI jobs, or chatops bots.
5. Add adapters and harden policy logic before enabling broader remediation automation.

## What problems SClaw solves
- **Speed:** reduces mean time to triage by structuring investigation steps.
- **Consistency:** enforces a repeatable incident response flow.
- **Safety:** adds policy and approval checks around risky operations.
- **Traceability:** captures findings and timeline for clean handoffs/postmortems.
- **Open-source usability:** supports local contributions without access to production tooling.

## Contributing
See `CONTRIBUTING.md` for development workflow, standards, and PR expectations.

## Roadmap
See `ROADMAP.md` for planned milestones and future features.
