use std::fmt::Write as FmtWrite;

use serde::Serialize;

use crate::apps::{InstalledSoftware, collect_app_catalog};
use crate::{ExitCode, brand};
use rz0_inventory_contract::ToolRecord;

pub const TOOLCHAIN_CONTRACT: &str = "toolchain_snapshot";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolchainReport {
    pub schema_version: u8,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub platform: &'static str,
    pub providers: Vec<ToolchainProvider>,
    pub tools: Vec<ToolchainTool>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolchainProvider {
    pub id: &'static str,
    pub label: &'static str,
    pub state: &'static str,
    pub observed_tool_count: usize,
    pub note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolchainTool {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source_id: String,
    pub provider: &'static str,
    pub state: &'static str,
}

const PROVIDERS: [(&str, &str, &str); 7] = [
    (
        "cargo",
        "Cargo",
        "Rust registry installs are reviewed through the shared updater contract.",
    ),
    (
        "rustup",
        "rustup",
        "Rust toolchain ownership remains separate from Cargo package ownership.",
    ),
    (
        "npm-prefix",
        "npm prefix",
        "Global npm records remain scoped to their discovered prefix.",
    ),
    (
        "homebrew",
        "Homebrew",
        "Homebrew formula and cask records remain manager-owned.",
    ),
    (
        "python",
        "Python",
        "pip and uv records remain explicit provider lanes.",
    ),
    (
        "self-updater",
        "Native self-updater",
        "Known self-updaters are observed only until an exact adapter exists.",
    ),
    (
        "native",
        "Native tool",
        "The source is visible, but no provider-specific update authority was inferred.",
    ),
];

pub fn toolchain_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [value] if matches!(value.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    match parse_format(args) {
        Ok(OutputFormat::Text) => match collect_toolchain_report() {
            Ok(report) => (ExitCode::Ok, render_text(&report), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
        },
        Ok(OutputFormat::Json) => match collect_toolchain_report() {
            Ok(report) => match serde_json::to_string_pretty(&report) {
                Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
                Err(error) => (
                    ExitCode::Usage,
                    String::new(),
                    format!("toolchain JSON rendering failed: {error}\n"),
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

pub fn collect_toolchain_report() -> Result<ToolchainReport, String> {
    let catalog = collect_app_catalog()?;
    Ok(toolchain_report_from_catalog(&catalog))
}

fn toolchain_report_from_catalog(catalog: &crate::apps::AppCatalog) -> ToolchainReport {
    let tools = toolchain_tools_from_catalog(catalog);

    let providers = PROVIDERS
        .iter()
        .map(|(id, label, note)| {
            let observed_tool_count = tools.iter().filter(|tool| tool.provider == *id).count();
            ToolchainProvider {
                id,
                label,
                state: if tools
                    .iter()
                    .any(|tool| tool.provider == *id && tool.state == "ready")
                {
                    "ready"
                } else {
                    "observed-only"
                },
                observed_tool_count,
                note,
            }
        })
        .collect();

    ToolchainReport {
        schema_version: 1,
        contract: TOOLCHAIN_CONTRACT,
        read_only: true,
        writes_attempted: false,
        platform: std::env::consts::OS,
        providers,
        tools,
        warnings: catalog.warnings.clone(),
    }
}

pub(crate) fn toolchain_tools_from_catalog(
    catalog: &crate::apps::AppCatalog,
) -> Vec<ToolchainTool> {
    let mut tools = catalog
        .apps
        .iter()
        .filter(|app| is_toolchain_software(app))
        .map(tool_from_app)
        .collect::<Vec<_>>();
    for tool in catalog
        .known_tools
        .iter()
        .filter(|tool| is_toolchain_record(tool))
        .map(tool_from_record)
    {
        if !tools
            .iter()
            .any(|existing| existing.id == tool.id && existing.source_id == tool.source_id)
        {
            tools.push(tool);
        }
    }
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
    tools
}

fn tool_from_app(app: &InstalledSoftware) -> ToolchainTool {
    let provider = toolchain_provider_id(&format!(
        "{} {} {} {:?}",
        app.id, app.name, app.source_id, app.identifiers
    ));
    ToolchainTool {
        id: app.id.clone(),
        name: app.name.clone(),
        version: app.version.clone(),
        source_id: app.source_id.clone(),
        provider,
        state: "ready",
    }
}

fn tool_from_record(tool: &ToolRecord) -> ToolchainTool {
    let source_id = tool
        .source_ids
        .first()
        .cloned()
        .unwrap_or_else(|| "known.executables".to_string());
    let provider = toolchain_provider_id(&format!(
        "{} {} {} {:?}",
        tool.id, tool.display_name, source_id, tool.source_ids
    ));
    ToolchainTool {
        id: tool.id.clone(),
        name: tool.display_name.clone(),
        version: tool.version.clone(),
        source_id,
        provider,
        state: "observed-only",
    }
}

pub(crate) fn toolchain_provider_id(value: &str) -> &'static str {
    provider_for_text(value)
}

pub fn is_toolchain_software(app: &InstalledSoftware) -> bool {
    is_toolchain_text(&format!(
        "{} {} {} {:?}",
        app.id, app.name, app.source_id, app.identifiers
    ))
}

pub(crate) fn is_toolchain_record(tool: &ToolRecord) -> bool {
    tool.category != "path_executable"
        && is_toolchain_text(&format!(
            "{} {} {} {:?}",
            tool.id, tool.display_name, tool.category, tool.source_ids
        ))
}

pub fn is_toolchain_text(value: &str) -> bool {
    [
        "cargo",
        "rustup",
        "codex",
        "claude",
        "gemini",
        "ollama",
        "open-webui",
        "warp",
        "cursor",
        "windsurf",
        "t3",
        "pi",
        "gsd",
        "omp",
        "hermes",
        "grok",
        "deno",
        "bun",
        "pnpm",
        "npm",
        "node",
        "uv",
        "mise",
        "asdf",
    ]
    .iter()
    .any(|token| {
        value
            .to_ascii_lowercase()
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|part| part == *token)
    })
}

fn provider_for_text(value: &str) -> &'static str {
    let value = value.to_ascii_lowercase();
    if value.contains("cargo") {
        "cargo"
    } else if value.contains("rustup") {
        "rustup"
    } else if value.contains("npm") || value.contains("node") || value.contains("pnpm") {
        "npm-prefix"
    } else if value.contains("homebrew") || value.contains("brew") {
        "homebrew"
    } else if value.contains("pip") || value.contains("uv") {
        "python"
    } else if ["warp", "cursor", "windsurf", "ollama", "t3"]
        .iter()
        .any(|token| value.contains(token))
    {
        "self-updater"
    } else {
        "native"
    }
}

fn render_text(report: &ToolchainReport) -> String {
    let mut output = format!("{} toolchain\n\n", brand::TITLE);
    let _ = writeln!(output, "mode: read-only local snapshot");
    let _ = writeln!(output, "platform: {}", report.platform);
    let _ = writeln!(output, "contract: {}", report.contract);
    let _ = writeln!(output, "writes attempted: no\n");
    output.push_str("providers:\n");
    for provider in &report.providers {
        let _ = writeln!(
            output,
            "  - {} [{}] · {} observed · {}",
            provider.label, provider.state, provider.observed_tool_count, provider.note
        );
    }
    output.push_str("\ntools:\n");
    if report.tools.is_empty() {
        output.push_str("  none observed\n");
    } else {
        for tool in &report.tools {
            let _ = writeln!(
                output,
                "  - {} {} · provider {} · source {}",
                tool.name,
                tool.version.as_deref().unwrap_or("version unknown"),
                tool.provider,
                tool.source_id
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
        "\nsafety: this command reads bounded local evidence only; it does not install, update, configure, or invoke a provider.\n",
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
                    return Err("toolchain --format requires text or json".to_string());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err(format!("unsupported toolchain output format '{value}'")),
                };
                index += 1;
            }
            value => return Err(format!("unsupported toolchain option '{value}'")),
        }
        index += 1;
    }
    Ok(format)
}

fn usage() -> String {
    "Usage: rz0 toolchain [--format text|json] [--json]\n\nReads bounded local software evidence and reports Rust, AI, and developer-tool provider posture. It never invokes a provider or performs a write.\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::{IdentityConfidence, InstallScope, SoftwareKind, UninstallOption};

    #[test]
    fn toolchain_classifier_is_token_bounded() {
        let app = InstalledSoftware {
            id: "package:npm-global:pi".to_string(),
            name: "pi".to_string(),
            version: Some("1.0.0".to_string()),
            source_id: "linux.path".to_string(),
            identifiers: Vec::new(),
            identity_group_id: "software.pi".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::PlatformPackage,
            scope: InstallScope::User,
            uninstall_option: UninstallOption::ManagerReview,
        };
        assert!(is_toolchain_software(&app));
        assert_eq!(provider_for_text("package:npm-global:pi"), "npm-prefix");
        assert_eq!(toolchain_provider_id("cargo:tool"), "cargo");
        assert!(!is_toolchain_text("application:capital"));
    }

    #[test]
    fn known_executable_inventory_is_bounded_to_named_tools() {
        let codex = ToolRecord {
            id: "codex".to_string(),
            display_name: "Codex".to_string(),
            category: "ai_tool".to_string(),
            executable_path: Some("/private/user/bin/codex".to_string()),
            version: None,
            source_ids: vec!["process.path".to_string()],
            confidence: "exact_path_match".to_string(),
            warnings: Vec::new(),
        };
        let wrapper = ToolRecord {
            id: "path.wrapper".to_string(),
            display_name: "codex-execve-wrapper".to_string(),
            category: "path_executable".to_string(),
            executable_path: Some("/private/user/bin/codex-execve-wrapper".to_string()),
            version: None,
            source_ids: vec!["known.executables".to_string()],
            confidence: "observed_path".to_string(),
            warnings: Vec::new(),
        };
        assert!(is_toolchain_record(&codex));
        assert!(!is_toolchain_record(&wrapper));
        let tool = tool_from_record(&codex);
        assert_eq!(tool.id, "codex");
        assert_eq!(tool.source_id, "process.path");
        assert_eq!(tool.state, "observed-only");
        let json = serde_json::to_string(&tool).expect("toolchain tool JSON");
        assert!(!json.contains("/private/user/bin"));
    }

    #[test]
    fn shared_toolchain_catalog_merge_counts_apps_and_known_tools_once() {
        let app = InstalledSoftware {
            id: "package:cargo:rz0".to_string(),
            name: "rz0".to_string(),
            version: Some("0.1.0".to_string()),
            source_id: "cargo".to_string(),
            identifiers: Vec::new(),
            identity_group_id: "software.rz0".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::PlatformPackage,
            scope: InstallScope::User,
            uninstall_option: UninstallOption::ManagerReview,
        };
        let known = ToolRecord {
            id: "codex".to_string(),
            display_name: "Codex".to_string(),
            category: "ai_tool".to_string(),
            executable_path: None,
            version: Some("1.0.0".to_string()),
            source_ids: vec!["process.path".to_string()],
            confidence: "exact_path_match".to_string(),
            warnings: Vec::new(),
        };
        let duplicate = ToolRecord {
            id: app.id.clone(),
            display_name: app.name.clone(),
            category: "package_manager".to_string(),
            executable_path: None,
            version: app.version.clone(),
            source_ids: vec![app.source_id.clone()],
            confidence: "exact_path_match".to_string(),
            warnings: Vec::new(),
        };
        let catalog = crate::apps::AppCatalog {
            schema_version: 1,
            contract: crate::apps::APP_CATALOG_CONTRACT,
            read_only: true,
            writes_attempted: false,
            platform: "test",
            source_count: 1,
            app_count: 1,
            service_count: 0,
            identity_group_count: 1,
            identity_groups: Vec::new(),
            apps: vec![app],
            known_tools: vec![known, duplicate],
            warnings: Vec::new(),
        };

        let tools = toolchain_tools_from_catalog(&catalog);
        assert_eq!(tools.len(), 2);
        assert_eq!(
            tools
                .iter()
                .filter(|tool| tool.id == "package:cargo:rz0")
                .count(),
            1
        );
        assert!(tools.iter().any(|tool| tool.id == "codex"));
    }

    #[test]
    fn toolchain_command_rejects_write_like_options() {
        let (code, _, error) = toolchain_command(&["--apply".to_string()]);
        assert_eq!(code, ExitCode::Usage);
        assert!(error.contains("unsupported toolchain option"));
    }
}
