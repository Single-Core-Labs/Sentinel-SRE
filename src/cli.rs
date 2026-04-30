use crate::config::RuntimeMode;
use crate::domain::{Environment, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub service: String,
    pub summary: String,
    pub severity: Severity,
    pub environment: Environment,
    pub mode: RuntimeMode,
    pub output: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct ReplConfig {
    pub service: String,
    pub severity: Severity,
    pub environment: Environment,
    pub mode: RuntimeMode,
    pub output: OutputFormat,
}

#[derive(Debug, Clone)]
pub enum CliCommand {
    Run(RunConfig),
    Repl(ReplConfig),
    Doctor(RuntimeMode),
    Benefits,
    Help,
}

pub fn parse_cli(args: &[String]) -> Result<CliCommand, String> {
    if args.is_empty() {
        return Ok(CliCommand::Repl(default_repl_config()));
    }

    match args[0].as_str() {
        "run" => Ok(CliCommand::Run(parse_run_config(args))),
        "repl" => Ok(CliCommand::Repl(parse_repl_config(args))),
        "doctor" => {
            let mode = RuntimeMode::from_str(&value_for(args, "--mode").unwrap_or_else(|| "real".to_string()));
            Ok(CliCommand::Doctor(mode))
        }
        "benefits" => Ok(CliCommand::Benefits),
        "help" | "--help" | "-h" => Ok(CliCommand::Help),
        _ => {
            if args[0].starts_with('-') {
                Err(format!("unknown option: {}", args[0]))
            } else {
                let summary = args.join(" ");
                Ok(CliCommand::Run(RunConfig {
                    service: value_for(args, "--service").unwrap_or_else(|| "payments-api".to_string()),
                    summary,
                    severity: Severity::from_str(
                        &value_for(args, "--severity").unwrap_or_else(|| "sev2".to_string()),
                    ),
                    environment: Environment::from_str(
                        &value_for(args, "--env").unwrap_or_else(|| "staging".to_string()),
                    ),
                    mode: RuntimeMode::from_str(
                        &value_for(args, "--mode").unwrap_or_else(|| "mock".to_string()),
                    ),
                    output: OutputFormat::from_str(
                        &value_for(args, "--output").unwrap_or_else(|| "text".to_string()),
                    ),
                }))
            }
        }
    }
}

fn parse_run_config(args: &[String]) -> RunConfig {
    RunConfig {
        service: value_for(args, "--service").unwrap_or_else(|| "payments-api".to_string()),
        summary: value_for(args, "--summary")
            .unwrap_or_else(|| "high latency and elevated error rate after deploy".to_string()),
        severity: Severity::from_str(
            &value_for(args, "--severity").unwrap_or_else(|| "sev2".to_string()),
        ),
        environment: Environment::from_str(
            &value_for(args, "--env").unwrap_or_else(|| "staging".to_string()),
        ),
        mode: RuntimeMode::from_str(&value_for(args, "--mode").unwrap_or_else(|| "mock".to_string())),
        output: OutputFormat::from_str(
            &value_for(args, "--output").unwrap_or_else(|| "text".to_string()),
        ),
    }
}

fn parse_repl_config(args: &[String]) -> ReplConfig {
    ReplConfig {
        service: value_for(args, "--service").unwrap_or_else(|| "payments-api".to_string()),
        severity: Severity::from_str(
            &value_for(args, "--severity").unwrap_or_else(|| "sev2".to_string()),
        ),
        environment: Environment::from_str(
            &value_for(args, "--env").unwrap_or_else(|| "staging".to_string()),
        ),
        mode: RuntimeMode::from_str(&value_for(args, "--mode").unwrap_or_else(|| "mock".to_string())),
        output: OutputFormat::from_str(
            &value_for(args, "--output").unwrap_or_else(|| "text".to_string()),
        ),
    }
}

fn default_repl_config() -> ReplConfig {
    ReplConfig {
        service: "payments-api".to_string(),
        severity: Severity::Sev2,
        environment: Environment::Staging,
        mode: RuntimeMode::Mock,
        output: OutputFormat::Text,
    }
}

fn value_for(args: &[String], key: &str) -> Option<String> {
    args.windows(2).find_map(|window| {
        if window[0] == key {
            Some(window[1].clone())
        } else {
            None
        }
    })
}
