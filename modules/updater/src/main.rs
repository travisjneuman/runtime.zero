use std::io::{self, IsTerminal, Read as _, Write as _};

use rz0_module_updater::{
    UpdaterFindingInput, build_serial_update_queue, build_update_action_plan, classify_updates,
};

const MAX_INPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;

fn main() {
    match run(std::env::args().skip(1).collect()) {
        Ok(output) => write_output(&output),
        Err(error) => exit_with(2, &error),
    }
}

fn run(arguments: Vec<String>) -> Result<String, String> {
    let command = parse_arguments(&arguments)?;
    if io::stdin().is_terminal() {
        return Err("updater input must be provided on standard input".to_string());
    }
    let mut input = Vec::new();
    io::stdin()
        .take(MAX_INPUT_BYTES.saturating_add(1))
        .read_to_end(&mut input)
        .map_err(|error| format!("read updater input: {error}"))?;
    if input.len() as u64 > MAX_INPUT_BYTES {
        return Err("updater input exceeds the foundation byte ceiling".to_string());
    }
    let input: UpdaterFindingInput = serde_json::from_slice(&input)
        .map_err(|error| format!("parse updater finding input: {error}"))?;
    let report = classify_updates(&input)?;
    if command.plan {
        let plan = build_update_action_plan(&input, &report)?;
        if command.queue {
            let queue = build_serial_update_queue(&plan)?;
            return if command.json {
                serde_json::to_string_pretty(&queue)
                    .map(|json| format!("{json}\n"))
                    .map_err(|error| format!("render updater queue: {error}"))
            } else {
                Ok(render_queue_text(&queue))
            };
        }
        return if command.json {
            serde_json::to_string_pretty(&plan)
                .map(|json| format!("{json}\n"))
                .map_err(|error| format!("render updater action plan: {error}"))
        } else {
            Ok(render_plan_text(&plan))
        };
    }
    if command.queue {
        return Err("--queue requires --plan".to_string());
    }
    if command.json {
        serde_json::to_string_pretty(&report)
            .map(|json| format!("{json}\n"))
            .map_err(|error| format!("render updater finding report: {error}"))
    } else {
        Ok(render_report_text(&report))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Command {
    json: bool,
    plan: bool,
    queue: bool,
}

fn parse_arguments(arguments: &[String]) -> Result<Command, String> {
    let mut command = Command {
        json: false,
        plan: false,
        queue: false,
    };
    let mut index = 0usize;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--json" => command.json = true,
            "--format" => {
                index += 1;
                command.json = match arguments.get(index).map(String::as_str) {
                    Some("text") => false,
                    Some("json") => true,
                    _ => return Err("--format requires text or json".to_string()),
                };
            }
            "--plan" => command.plan = true,
            "--queue" => command.queue = true,
            "--help" | "-h" | "help" => return Err(usage().to_string()),
            value if value.starts_with("--format=") => {
                if value != "--format=json" && value != "--format=text" {
                    return Err("unsupported updater output format".to_string());
                }
                command.json = value.ends_with("=json");
            }
            value => return Err(format!("unsupported updater option '{value}'")),
        }
        index += 1;
    }
    Ok(command)
}

fn render_report_text(report: &rz0_finding_contract::FindingReport) -> String {
    format!(
        "runtime.zero updater finding report\n\ncontract: {}\nreport_id: {}\nplatform: {}\nread_only: yes\nwrites_attempted: no\nupdate_candidates: {}\nblocked: {}\n\nNo manager command was executed.\n",
        report.contract,
        report.report_id,
        report.platform,
        report.summary.manager_action_candidate_count,
        report.summary.blocked_count,
    )
}

fn render_queue_text(queue: &rz0_module_updater::SerialUpdateQueuePlan) -> String {
    format!(
        "runtime.zero serial update queue\n\nqueue_id: {}\nactions: {}\ndry_run: yes\nwrites_attempted: no\nexecution_authorized: no\n\nThe queue is review-only and pauses on failure, drift, cancellation, or recovery.\n",
        queue.queue_id,
        queue.items.len(),
    )
}

fn render_plan_text(plan: &rz0_action_plan::ActionPlan) -> String {
    let planned = plan
        .actions
        .iter()
        .filter(|action| action.disposition == rz0_action_plan::ActionDisposition::Planned)
        .count();
    let blocked = plan.actions.len().saturating_sub(planned);
    format!(
        "runtime.zero updater action plan\n\nplan_id: {}\nmodule: {}\ndry_run: yes\nwrites_attempted: no\nplanned_actions: {}\nblocked_actions: {}\n\nNo manager command was executed.\n",
        plan.plan_id, plan.module_id, planned, blocked
    )
}

fn usage() -> &'static str {
    "Usage: rz0-updater [--format text|json] [--plan] < updater-finding-input.json\n\nClassifies installed-only update evidence and optionally emits a foundation dry-run action plan.\nNo manager command, network request, or write is performed."
}

fn write_output(output: &str) {
    if let Err(error) = io::stdout().write_all(output.as_bytes()) {
        exit_with(3, &format!("write updater output: {error}"));
    }
}

fn exit_with(code: i32, message: &str) -> ! {
    let _ = writeln!(io::stderr(), "rz0-updater: {message}");
    std::process::exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_bounded_safe_options() {
        assert_eq!(
            parse_arguments(&[
                "--format".to_string(),
                "json".to_string(),
                "--plan".to_string()
            ])
            .unwrap(),
            Command {
                json: true,
                plan: true,
                queue: false,
            }
        );
        assert!(
            !parse_arguments(&["--format=text".to_string()])
                .unwrap()
                .json
        );
        assert!(parse_arguments(&["--delete".to_string()]).is_err());
        assert!(parse_arguments(&["--format=yaml".to_string()]).is_err());
    }
}
