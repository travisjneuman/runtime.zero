//! Rust-owned AIUP-facing toolchain review.
//!
//! This surface is deliberately narrower than the standalone AIUP tool. It
//! owns local identity normalization and provider posture, while the updater
//! module remains the only place that can form a manager action plan. The
//! command never invokes AIUP or another provider and never writes state.

use std::fmt::Write as FmtWrite;

use serde::Serialize;

use crate::apps::{AppCatalog, InstalledSoftware, collect_app_catalog};
use crate::toolchain::toolchain_provider_id;
use crate::{ExitCode, brand};

pub const AIUP_CONTRACT: &str = "ai_toolchain_snapshot";
pub const AIUP_CAPABILITY_ID: &str = "first-party.updater.ai-toolchain";

const AI_TOOL_SPECS: &[&str] = &[
    "claude",
    "codex",
    "cursor",
    "gemini",
    "grok",
    "gsd",
    "hermes",
    "kilo",
    "ollama",
    "omp",
    "open-webui",
    "opencode",
    "pi",
    "t3",
    "warp",
    "windsurf",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiupReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub capability_id: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub platform: &'static str,
    pub orchestrator: AiupOrchestrator,
    pub providers: Vec<AiupProvider>,
    pub tools: Vec<AiupTool>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiupOrchestrator {
    pub state: &'static str,
    pub version: Option<String>,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiupProvider {
    pub id: &'static str,
    pub label: &'static str,
    pub state: &'static str,
    pub observed_tool_count: usize,
    pub action_boundary: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiupTool {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source_id: String,
    pub provider: &'static str,
    pub state: &'static str,
    pub next_step: &'static str,
}

pub fn aiup_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [value] if matches!(value.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    match parse_format(args) {
        Ok(OutputFormat::Text) => match collect_aiup_report() {
            Ok(report) => (ExitCode::Ok, render_text(&report), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
        },
        Ok(OutputFormat::Json) => match collect_aiup_report() {
            Ok(report) => match serde_json::to_string_pretty(&report) {
                Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
                Err(error) => (
                    ExitCode::Usage,
                    String::new(),
                    format!("AIUP JSON rendering failed: {error}\n"),
                ),
            },
            Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
        },
        Err(error) => (
            ExitCode::Usage,
            String::new(),
            format!("{error}\n{}", usage()),
        ),
    }
}

pub fn collect_aiup_report() -> Result<AiupReport, String> {
    let catalog = collect_app_catalog()?;
    Ok(report_from_catalog(&catalog))
}

fn report_from_catalog(catalog: &AppCatalog) -> AiupReport {
    let orchestrator = catalog
        .apps
        .iter()
        .find(|app| is_aiup_orchestrator(app))
        .map(|app| AiupOrchestrator {
            state: "observed-only",
            version: app.version.clone(),
            source_id: Some(app.source_id.clone()),
        })
        .unwrap_or(AiupOrchestrator {
            state: "not-observed",
            version: None,
            source_id: None,
        });

    let mut tools = catalog
        .apps
        .iter()
        .filter(|app| is_ai_tool_software(app))
        .map(aiup_tool_from_app)
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| {
        left.provider
            .cmp(right.provider)
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
            .then_with(|| left.id.cmp(&right.id))
    });

    let providers = [
        ("aiup", "AIUP", "catalog evidence only"),
        (
            "npm-prefix",
            "npm prefix",
            "review through the shared updater lane",
        ),
        (
            "homebrew",
            "Homebrew",
            "review through the shared updater lane",
        ),
        (
            "self-updater",
            "Native self-updater",
            "provider channel must be reviewed separately",
        ),
        (
            "native",
            "Native tool",
            "no update authority inferred from discovery",
        ),
    ]
    .into_iter()
    .map(|(id, label, action_boundary)| {
        let count = tools.iter().filter(|tool| tool.provider == id).count();
        AiupProvider {
            id,
            label,
            state: if count == 0 {
                "not-observed"
            } else if id == "aiup" || id == "self-updater" || id == "native" {
                "observed-only"
            } else {
                "provider-review"
            },
            observed_tool_count: count,
            action_boundary,
        }
    })
    .collect();

    let mut warnings = catalog.warnings.clone();
    if tools.is_empty() {
        warnings
            .push("no supported AI tools were observed in the local software catalog".to_string());
    }
    if orchestrator.state == "not-observed" {
        warnings.push("AIUP orchestrator was not observed; provider actions remain unavailable from this snapshot".to_string());
    }

    AiupReport {
        schema_version: 1,
        contract: AIUP_CONTRACT,
        capability_id: AIUP_CAPABILITY_ID,
        read_only: true,
        writes_attempted: false,
        platform: std::env::consts::OS,
        orchestrator,
        providers,
        tools,
        warnings,
    }
}

fn aiup_tool_from_app(app: &InstalledSoftware) -> AiupTool {
    let provider = if app.source_id.to_ascii_lowercase().contains("aiup") {
        "aiup"
    } else {
        toolchain_provider_id(&format!(
            "{} {} {} {:?}",
            app.id, app.name, app.source_id, app.identifiers
        ))
    };
    let (state, next_step) = match provider {
        "npm-prefix" | "homebrew" => ("observed", "review provider availability with rz0 updates"),
        "aiup" => ("observed-only", "review AIUP evidence through rz0 updates"),
        _ => (
            "observed-only",
            "provider-specific action is not established",
        ),
    };
    AiupTool {
        id: app.id.clone(),
        name: app.name.clone(),
        version: app.version.clone(),
        source_id: app.source_id.clone(),
        provider,
        state,
        next_step,
    }
}

pub fn is_ai_tool_software(app: &InstalledSoftware) -> bool {
    is_ai_tool_text(&format!(
        "{} {} {} {:?}",
        app.id, app.name, app.source_id, app.identifiers
    ))
}

pub fn is_ai_tool_text(value: &str) -> bool {
    AI_TOOL_SPECS.iter().any(|id| bounded_match(value, id))
}

fn is_aiup_orchestrator(app: &InstalledSoftware) -> bool {
    app.id.eq_ignore_ascii_case("aiup") || app.name.eq_ignore_ascii_case("aiup")
}

fn bounded_match(value: &str, needle: &str) -> bool {
    let value = value.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    value.match_indices(&needle).any(|(index, _)| {
        let before = value[..index].chars().next_back();
        let after = value[index + needle.len()..].chars().next();
        before.is_none_or(|character| !character.is_ascii_alphanumeric())
            && after.is_none_or(|character| !character.is_ascii_alphanumeric())
    })
}

fn render_text(report: &AiupReport) -> String {
    let mut output = format!("{} aiup\n\n", brand::TITLE);
    let _ = writeln!(output, "mode: read-only local snapshot");
    let _ = writeln!(output, "capability: {}", report.capability_id);
    let _ = writeln!(output, "platform: {}", report.platform);
    let _ = writeln!(output, "contract: {}", report.contract);
    let _ = writeln!(
        output,
        "AIUP orchestrator: {}{}\n",
        report.orchestrator.state,
        report
            .orchestrator
            .version
            .as_deref()
            .map(|version| format!(" · {version}"))
            .unwrap_or_default()
    );
    output.push_str("providers:\n");
    for provider in &report.providers {
        let _ = writeln!(
            output,
            "  - {} [{}] · {} observed · {}",
            provider.label, provider.state, provider.observed_tool_count, provider.action_boundary
        );
    }
    output.push_str("\ntools:\n");
    if report.tools.is_empty() {
        output.push_str("  none observed\n");
    } else {
        for tool in &report.tools {
            let _ = writeln!(
                output,
                "  - {} {} · provider {} · source {} · {}",
                tool.name,
                tool.version.as_deref().unwrap_or("version unknown"),
                tool.provider,
                tool.source_id,
                tool.state
            );
        }
    }
    if !report.warnings.is_empty() {
        output.push_str("\nwarnings:\n");
        for warning in &report.warnings {
            let _ = writeln!(output, "  - {warning}");
        }
    }
    output.push_str(
        "\nboundary: Rust owns discovery, identity, redaction, and review posture; provider updates remain behind rz0 updates plans and confirmation.\n",
    );
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

fn parse_format(args: &[String]) -> Result<OutputFormat, String> {
    let mut format = OutputFormat::Text;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" | "help" if args.len() == 1 => return Err(usage()),
            "--json" => format = OutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err("aiup --format requires text or json".to_string());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err(format!("unsupported AIUP output format '{value}'")),
                };
                index += 1;
            }
            value => return Err(format!("unsupported aiup option '{value}'")),
        }
        index += 1;
    }
    Ok(format)
}

fn usage() -> String {
    "Usage: rz0 aiup [--format text|json] [--json]\n\nReads bounded local AI-tool evidence and reports AIUP/provider posture. It never invokes AIUP, installs or updates a tool, configures a provider, or writes state.\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::{IdentityConfidence, InstallScope, SoftwareKind, UninstallOption};

    fn app(id: &str, name: &str, source_id: &str) -> InstalledSoftware {
        InstalledSoftware {
            id: id.to_string(),
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            source_id: source_id.to_string(),
            identifiers: Vec::new(),
            identity_group_id: format!("software.{id}"),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::PlatformPackage,
            scope: InstallScope::User,
            uninstall_option: UninstallOption::ManagerReview,
        }
    }

    #[test]
    fn ai_tool_classifier_accepts_bounded_names_and_composites() {
        assert!(is_ai_tool_text("package:npm-global:codex"));
        assert!(is_ai_tool_text("application:open-webui"));
        assert!(!is_ai_tool_text("application:codexical"));
        assert!(!is_ai_tool_text("application:my-pitoolbox"));
    }

    #[test]
    fn report_separates_orchestrator_from_managed_tools() {
        let catalog = AppCatalog {
            schema_version: 1,
            contract: crate::apps::APP_CATALOG_CONTRACT,
            read_only: true,
            writes_attempted: false,
            platform: "test",
            source_count: 1,
            app_count: 3,
            service_count: 0,
            identity_group_count: 3,
            identity_groups: Vec::new(),
            apps: vec![
                app("aiup", "AIUP", "test.path"),
                app("package:npm-global:codex", "Codex", "npm.global"),
                app("aiup-managed:hermes", "Hermes", "aiup.catalog"),
            ],
            warnings: Vec::new(),
        };
        let report = report_from_catalog(&catalog);
        assert_eq!(report.orchestrator.state, "observed-only");
        assert_eq!(report.tools.len(), 2);
        assert_eq!(report.tools[0].provider, "aiup");
        assert_eq!(report.tools[1].provider, "npm-prefix");
        assert!(report.read_only);
        assert!(!report.writes_attempted);
    }

    #[test]
    fn command_rejects_write_like_options() {
        let (code, _, error) = aiup_command(&["--apply".to_string()]);
        assert_eq!(code, ExitCode::Usage);
        assert!(error.contains("unsupported aiup option"));
    }
}
