use std::env;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use sentinel_sre::approval::RuleBasedApprovalGate;
use sentinel_sre::cli::{parse_cli, CliCommand, OutputFormat, ReplConfig, RunConfig};
use sentinel_sre::config::{ProviderConfig, RuntimeMode};
use sentinel_sre::domain::{Environment, IncidentInput, IncidentReport, Severity};
use sentinel_sre::engine::SreAgent;
use sentinel_sre::memory::KnowledgeMemory;
use sentinel_sre::policy::PolicyEngine;
use sentinel_sre::tools::{MockSreTools, RealSreTools, Toolset};
use serde_json::json;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = match parse_cli(&args) {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("{}", err);
            print_help();
            return;
        }
    };

    match command {
        CliCommand::Run(run) => {
            let report = run_incident(&run);
            print_report(&report, run.output);
        }
        CliCommand::Repl(config) => run_repl(config),
        CliCommand::Doctor(mode) => run_doctor(mode),
        CliCommand::Benefits => print_benefits(),
        CliCommand::Help => print_help(),
    }
}

fn run_incident(config: &RunConfig) -> IncidentReport {
    let incident = IncidentInput {
        incident_id: format!("inc-{}", unix_seconds_now()),
        service: config.service.clone(),
        summary: config.summary.clone(),
        severity: config.severity,
        environment: config.environment,
    };

    let tools = build_toolset(config.mode);
    let agent = SreAgent::new(
        tools,
        PolicyEngine::default(),
        RuleBasedApprovalGate::default(),
        KnowledgeMemory::with_defaults(),
    );
    agent.run(incident)
}

fn run_repl(mut config: ReplConfig) {
    println!("Sentinel-SRE interactive mode");
    println!("Type an incident summary to run analysis.");
    println!("Use /help for commands, /exit to quit.");

    loop {
        print!("sentinel> ");
        if io::stdout().flush().is_err() {
            eprintln!("failed to flush prompt");
            return;
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => return,
            Ok(_) => {}
            Err(err) => {
                eprintln!("input error: {}", err);
                return;
            }
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        if line.eq_ignore_ascii_case("/exit") || line.eq_ignore_ascii_case("/quit") {
            println!("bye");
            return;
        }
        if line.eq_ignore_ascii_case("/help") {
            print_repl_help();
            continue;
        }
        if line.eq_ignore_ascii_case("/benefits") {
            print_benefits();
            continue;
        }
        if line.eq_ignore_ascii_case("/status") {
            println!(
                "service={} severity={:?} env={:?} mode={:?} output={:?}",
                config.service, config.severity, config.environment, config.mode, config.output
            );
            continue;
        }
        if line.eq_ignore_ascii_case("/doctor") {
            run_doctor(config.mode);
            continue;
        }
        if let Some(rest) = line.strip_prefix("/set ") {
            let mut parts = rest.splitn(2, ' ');
            let key = parts.next().unwrap_or_default().trim();
            let value = parts.next().unwrap_or_default().trim();
            if value.is_empty() {
                println!("usage: /set <service|severity|env|mode|output> <value>");
                continue;
            }
            match key {
                "service" => config.service = value.to_string(),
                "severity" => config.severity = Severity::from_str(value),
                "env" => config.environment = Environment::from_str(value),
                "mode" => config.mode = RuntimeMode::from_str(value),
                "output" => config.output = OutputFormat::from_str(value),
                _ => {
                    println!("unknown key '{}'", key);
                    continue;
                }
            }
            println!("updated {}={}", key, value);
            continue;
        }

        let run = RunConfig {
            service: config.service.clone(),
            summary: line.to_string(),
            severity: config.severity,
            environment: config.environment,
            mode: config.mode,
            output: config.output,
        };
        let report = run_incident(&run);
        print_report(&report, config.output);
    }
}

fn run_doctor(mode: RuntimeMode) {
    println!("Sentinel-SRE doctor");
    println!("mode: {:?}", mode);
    match mode {
        RuntimeMode::Mock => {
            println!("mock mode ready");
            println!("no external integrations required");
        }
        RuntimeMode::Real => {
            let config = ProviderConfig::from_env();
            println!(
                "SC_PROMETHEUS_URL: {}",
                summarize_presence(&config.prometheus_url)
            );
            println!("SC_LOKI_URL: {}", summarize_presence(&config.loki_url));
            println!("SC_K8S_API_URL: {}", summarize_presence(&config.k8s_api_url));
            println!(
                "SC_K8S_NAMESPACE: {}",
                summarize_presence(&config.k8s_namespace)
            );
            println!(
                "SC_SERVICE_LABEL_KEY: {}",
                summarize_presence(&config.service_label_key)
            );
            println!(
                "SC_API_BEARER_TOKEN: {}",
                summarize_optional(&config.bearer_token)
            );
            println!(
                "SC_REMEDIATION_WEBHOOK_URL: {}",
                summarize_optional(&config.remediation_webhook_url)
            );
            println!("real mode configured (endpoint reachability depends on network and credentials)");
        }
    }
}

fn summarize_presence(value: &str) -> &'static str {
    if value.trim().is_empty() {
        "missing"
    } else {
        "set"
    }
}

fn summarize_optional(value: &Option<String>) -> &'static str {
    if value.is_some() {
        "set"
    } else {
        "not set"
    }
}

fn build_toolset(mode: RuntimeMode) -> Toolset {
    match mode {
        RuntimeMode::Mock => Toolset::Mock(MockSreTools::default()),
        RuntimeMode::Real => {
            let config = ProviderConfig::from_env();
            match RealSreTools::from_config(config) {
                Ok(real) => Toolset::Real(real),
                Err(err) => {
                    eprintln!(
                        "failed to initialize real tools ({}), falling back to mock mode",
                        err
                    );
                    Toolset::Mock(MockSreTools::default())
                }
            }
        }
    }
}

fn print_report(report: &IncidentReport, output: OutputFormat) {
    match output {
        OutputFormat::Text => print_report_text(report),
        OutputFormat::Json => print_report_json(report),
    }
}

fn print_report_text(report: &IncidentReport) {
    println!("final_state: {:?}", report.final_state);
    println!("findings:");
    for finding in &report.findings {
        println!("  - {}", finding);
    }

    if let Some(plan) = &report.selected_mitigation {
        println!("selected_mitigation:");
        println!("  action: {:?}", plan.action);
        println!("  target: {}", plan.target);
        println!("  risk: {:?}", plan.risk_level);
        println!("  reason: {}", plan.reason);
    }

    println!("timeline:");
    for entry in &report.timeline {
        println!("  - {}", entry);
    }
}

fn print_report_json(report: &IncidentReport) {
    let mitigation = report.selected_mitigation.as_ref().map(|plan| {
        json!({
            "action": format!("{:?}", plan.action),
            "target": plan.target.clone(),
            "risk": format!("{:?}", plan.risk_level),
            "reason": plan.reason.clone(),
            "blast_radius_estimate": plan.blast_radius_estimate.clone(),
            "rollback_steps": plan.rollback_steps.clone(),
        })
    });

    let payload = json!({
        "final_state": format!("{:?}", report.final_state),
        "findings": report.findings.clone(),
        "timeline": report.timeline.clone(),
        "selected_mitigation": mitigation,
    });
    match serde_json::to_string_pretty(&payload) {
        Ok(content) => println!("{}", content),
        Err(err) => eprintln!("failed to render JSON output: {}", err),
    }
}

fn print_benefits() {
    println!("How Sentinel-SRE CLI helps your developers and SREs:");
    println!("1) Faster incident triage from terminal commands or interactive mode.");
    println!("2) Consistent response flow with policy and approval guardrails.");
    println!("3) Shared audit timeline for handoffs, retros, and postmortems.");
    println!("4) Mock mode for contributor onboarding without infra access.");
    println!("5) Real mode to integrate observability and remediation pipelines.");
}

fn print_help() {
    println!("Sentinel-SRE CLI");
    println!("Usage:");
    println!("  sentinel-sre run [--service <name>] [--summary <text>] [--severity <sev1|sev2|sev3|sev4>] [--env <prod|staging|dev>] [--mode <mock|real>] [--output <text|json>]");
    println!("  sentinel-sre repl [--service <name>] [--severity <...>] [--env <...>] [--mode <...>] [--output <...>]");
    println!("  sentinel-sre doctor [--mode <mock|real>]");
    println!("  sentinel-sre benefits");
    println!("  sentinel-sre help");
    println!();
    print_repl_help();
}

fn print_repl_help() {
    println!("Interactive commands:");
    println!("  /help");
    println!("  /status");
    println!("  /set service <name>");
    println!("  /set severity <sev1|sev2|sev3|sev4>");
    println!("  /set env <prod|staging|dev>");
    println!("  /set mode <mock|real>");
    println!("  /set output <text|json>");
    println!("  /doctor");
    println!("  /benefits");
    println!("  /exit");
    println!("Any non-slash line is treated as incident summary input.");
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
