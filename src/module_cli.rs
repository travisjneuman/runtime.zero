use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::builtin_modules::{BuiltinLifecycleMode, BuiltinLifecycleRequest, BuiltinOperation};
use crate::module_lifecycle_exec::{
    LifecycleMode, LifecycleOperation, LifecycleOperationLabel, LifecycleRequest,
};
use crate::{
    ExitCode, brand, module_install_plan, module_manifest, module_registry, module_status,
    module_trust_cli, module_validation,
};
use rz0_module_lifecycle::{ModuleLifecycleOperation, ModuleLifecycleState, module_lifecycle_plan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

enum ModulesAction {
    Help,
    List {
        format: OutputFormat,
        from: Option<String>,
    },
    Status {
        format: OutputFormat,
        store_root: Option<PathBuf>,
    },
    Validate {
        path: String,
        format: OutputFormat,
    },
    InstallDryRun {
        path: String,
        format: OutputFormat,
    },
    InstallDeveloperTrial {
        request: crate::module_stage::DeveloperStageRequest,
        format: OutputFormat,
    },
    InstallSigned {
        request: crate::module_stage::SignedStageRequest,
        format: OutputFormat,
    },
    LifecycleExecute {
        request: LifecycleRequest,
        format: OutputFormat,
    },
    BuiltinLifecycle {
        request: BuiltinLifecycleRequest,
        format: OutputFormat,
    },
    InvokeDeveloperTrial {
        request: crate::module_invoke::DeveloperInvocationRequest,
        format: OutputFormat,
    },
    InvokeSigned {
        request: crate::module_invoke::SignedInvocationRequest,
        format: OutputFormat,
    },
    LifecyclePlan {
        operation: ModuleLifecycleOperation,
        module_id: String,
        from_state: ModuleLifecycleState,
        to_state: ModuleLifecycleState,
        from_version: Option<String>,
        to_version: Option<String>,
        transition_id: Option<String>,
        format: OutputFormat,
    },
    Trust {
        args: Vec<String>,
    },
}

struct LifecyclePlanRenderRequest {
    operation: ModuleLifecycleOperation,
    module_id: String,
    from_state: ModuleLifecycleState,
    to_state: ModuleLifecycleState,
    from_version: Option<String>,
    to_version: Option<String>,
    transition_id: Option<String>,
}

pub fn modules_command(args: &[String]) -> (ExitCode, String, String) {
    match parse_modules_args(args) {
        Ok(ModulesAction::Help) => (ExitCode::Ok, modules_usage(), String::new()),
        Ok(ModulesAction::List { format, from }) => render_modules(format, from.as_deref()),
        Ok(ModulesAction::Status { format, store_root }) => {
            render_module_status(format, store_root)
        }
        Ok(ModulesAction::Validate { path, format }) => render_validation(format, &path),
        Ok(ModulesAction::InstallDryRun { path, format }) => render_install_plan(format, &path),
        Ok(ModulesAction::InstallDeveloperTrial { request, format }) => {
            render_developer_stage(format, &request)
        }
        Ok(ModulesAction::InstallSigned { request, format }) => {
            render_signed_stage(format, &request)
        }
        Ok(ModulesAction::LifecycleExecute { request, format }) => {
            render_lifecycle_execution(format, &request)
        }
        Ok(ModulesAction::BuiltinLifecycle { request, format }) => {
            render_builtin_lifecycle(format, &request)
        }
        Ok(ModulesAction::InvokeDeveloperTrial { request, format }) => {
            render_developer_invocation(format, &request)
        }
        Ok(ModulesAction::InvokeSigned { request, format }) => {
            render_signed_invocation(format, &request)
        }
        Ok(ModulesAction::LifecyclePlan {
            operation,
            module_id,
            from_state,
            to_state,
            from_version,
            to_version,
            transition_id,
            format,
        }) => render_lifecycle_plan(
            format,
            LifecyclePlanRenderRequest {
                operation,
                module_id,
                from_state,
                to_state,
                from_version,
                to_version,
                transition_id,
            },
        ),
        Ok(ModulesAction::Trust { args }) => module_trust_cli::trust_command(&args),
        Err(err) => (ExitCode::Usage, String::new(), err),
    }
}

pub fn modules_text() -> String {
    modules_text_from(None)
}

pub fn modules_json() -> Result<String, String> {
    modules_json_from(None)
}

fn parse_modules_args(args: &[String]) -> Result<ModulesAction, String> {
    match args.first().map(String::as_str) {
        Some(flag) if matches!(flag, "--help" | "-h" | "help") && args.len() == 1 => {
            Ok(ModulesAction::Help)
        }
        Some("validate") => parse_validate_args(&args[1..]),
        Some("install") => parse_install_args(&args[1..]),
        Some("invoke") => parse_invoke_args(&args[1..]),
        Some("enable") => parse_lifecycle_execute_args(&args[1..], LifecycleOperationLabel::Enable),
        Some("disable") => {
            parse_lifecycle_execute_args(&args[1..], LifecycleOperationLabel::Disable)
        }
        Some("update") => parse_lifecycle_execute_args(&args[1..], LifecycleOperationLabel::Update),
        Some("uninstall") => {
            parse_lifecycle_execute_args(&args[1..], LifecycleOperationLabel::Uninstall)
        }
        Some("recover") => {
            parse_lifecycle_execute_args(&args[1..], LifecycleOperationLabel::Recover)
        }
        Some("builtin") => parse_builtin_args(&args[1..]),
        Some("status") => parse_status_args(&args[1..]),
        Some("lifecycle-plan") => parse_lifecycle_plan_args(&args[1..]),
        Some("trust") => Ok(ModulesAction::Trust {
            args: args[1..].to_vec(),
        }),
        _ => parse_list_args(args),
    }
}

fn parse_status_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut store_root = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            "--store-root" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(status_usage());
                };
                if store_root.is_some() {
                    return Err("module status store root was provided more than once".to_string());
                }
                store_root = Some(resolve_store_root(value)?);
                index += 1;
            }
            _ => return Err(status_usage()),
        }
        index += 1;
    }
    Ok(ModulesAction::Status { format, store_root })
}

fn resolve_store_root(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains("://") {
        return Err("module status store root must be a local filesystem path".to_string());
    }
    let path = PathBuf::from(trimmed);
    let absolute = if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map_err(|err| format!("failed to resolve current directory: {err}"))?
            .join(path)
    };
    if absolute.exists() {
        fs::canonicalize(&absolute)
            .map_err(|err| format!("failed to canonicalize module status store root: {err}"))
    } else {
        Ok(absolute)
    }
}

fn parse_list_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut from = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = OutputFormat::Json,
            "--format" => {
                format = parse_format(args, &mut index)?;
            }
            "--from" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(usage_error(args));
                };
                if from.replace(path.clone()).is_some() {
                    return Err("module source directory was provided more than once".to_string());
                }
                index += 1;
            }
            _ => return Err(usage_error(args)),
        }
        index += 1;
    }
    Ok(list_action(format, from.as_ref()))
}

fn parse_validate_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut path = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            value if path.is_none() => path = Some(value.to_string()),
            _ => return Err(usage_error(args)),
        }
        index += 1;
    }
    let Some(path) = path else {
        return Err(usage_error(args));
    };
    Ok(validate_action(&path, format))
}

fn parse_install_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut mode = None;
    let mut developer_trial = false;
    let mut signed = false;
    let mut developer_promote = false;
    let mut path = None;
    let mut signature = None;
    let mut trusted_test_key = None;
    let mut store_root = None;
    let mut challenge_issued_unix_seconds = None;
    let mut confirmation = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--developer-trial" if !developer_trial => developer_trial = true,
            "--developer-trial" => return Err(install_usage()),
            "--signed" if !signed => signed = true,
            "--signed" => return Err(install_usage()),
            "--developer-promote" if !developer_promote => developer_promote = true,
            "--developer-promote" => return Err(install_usage()),
            "--dry-run" if mode.is_none() => mode = Some(false),
            "--dry-run" => return Err(install_usage()),
            "--apply" if mode.is_none() => mode = Some(true),
            "--apply" => return Err(install_usage()),
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            "--signature" => set_option(&mut signature, args, &mut index, "signature path")?,
            "--trusted-test-key" => set_option(
                &mut trusted_test_key,
                args,
                &mut index,
                "trusted test key path",
            )?,
            "--store-root" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(install_usage());
                };
                if store_root.is_some() {
                    return Err(
                        "module developer trial store root was provided more than once".to_string(),
                    );
                }
                store_root = Some(resolve_store_root(value)?);
                index += 1;
            }
            "--challenge-issued-unix-seconds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(install_usage());
                };
                if challenge_issued_unix_seconds.is_some() {
                    return Err(
                        "module developer trial challenge time was provided more than once"
                            .to_string(),
                    );
                }
                challenge_issued_unix_seconds =
                    Some(value.parse::<u64>().map_err(|_| {
                        "challenge-issued-unix-seconds must be an integer".to_string()
                    })?);
                index += 1;
            }
            "--confirm" => set_option(&mut confirmation, args, &mut index, "confirmation phrase")?,
            value if path.is_none() => path = Some(value.to_string()),
            _ => return Err(install_usage()),
        }
        index += 1;
    }
    let Some(path) = path else {
        return Err(install_usage());
    };
    if developer_trial || signed {
        let Some(apply) = mode else {
            return Err(install_usage());
        };
        let Some(signature) = signature else {
            return Err(format!(
                "signed module install requires --signature <envelope.json>\n\n{}",
                install_usage()
            ));
        };
        let Some(trusted_test_key) = trusted_test_key else {
            return Err(format!(
                "signed module install requires --trusted-key <key.json>\n\n{}",
                install_usage()
            ));
        };
        let Some(store_root) = store_root else {
            return Err(format!(
                "signed module install requires --store-root <path>\n\n{}",
                install_usage()
            ));
        };
        let stage_mode = if apply {
            let Some(issued) = challenge_issued_unix_seconds else {
                return Err(format!(
                    "developer trial apply requires --challenge-issued-unix-seconds <seconds>\n\n{}",
                    install_usage()
                ));
            };
            let Some(confirmation) = confirmation.clone() else {
                return Err(format!(
                    "developer trial apply requires --confirm <exact-phrase>\n\n{}",
                    install_usage()
                ));
            };
            crate::module_stage::DeveloperStageMode::Apply {
                challenge_issued_unix_seconds: issued,
                confirmation: confirmation.clone(),
                publish_installed: developer_promote,
            }
        } else {
            if challenge_issued_unix_seconds.is_some() || confirmation.is_some() {
                return Err(
                    "developer trial dry-run does not accept apply confirmation arguments"
                        .to_string(),
                );
            }
            crate::module_stage::DeveloperStageMode::DryRun {
                publish_installed: developer_promote,
            }
        };
        if signed && developer_promote {
            return Err("--developer-promote is only valid with --developer-trial".to_string());
        }
        if signed {
            let mode = if apply {
                let Some(issued) = challenge_issued_unix_seconds else {
                    return Err(format!(
                        "signed module install apply requires --challenge-issued-unix-seconds <seconds>\n\n{}",
                        install_usage()
                    ));
                };
                let Some(confirmation) = confirmation else {
                    return Err(format!(
                        "signed module install apply requires --confirm <exact-phrase>\n\n{}",
                        install_usage()
                    ));
                };
                crate::module_stage::DeveloperStageMode::Apply {
                    challenge_issued_unix_seconds: issued,
                    confirmation,
                    publish_installed: true,
                }
            } else {
                crate::module_stage::DeveloperStageMode::DryRun {
                    publish_installed: true,
                }
            };
            return Ok(ModulesAction::InstallSigned {
                request: crate::module_stage::SignedStageRequest {
                    package_path: PathBuf::from(path),
                    signature_path: PathBuf::from(signature),
                    trusted_key_path: PathBuf::from(trusted_test_key),
                    store_root,
                    mode,
                },
                format,
            });
        }
        return Ok(ModulesAction::InstallDeveloperTrial {
            request: crate::module_stage::DeveloperStageRequest {
                package_path: PathBuf::from(path),
                signature_path: PathBuf::from(signature),
                trusted_key_path: PathBuf::from(trusted_test_key),
                store_root,
                mode: stage_mode,
            },
            format,
        });
    }
    if developer_promote {
        return Err("--developer-promote requires --developer-trial".to_string());
    }
    if mode != Some(false)
        || signature.is_some()
        || trusted_test_key.is_some()
        || store_root.is_some()
        || challenge_issued_unix_seconds.is_some()
        || confirmation.is_some()
    {
        return Err(install_dry_run_usage());
    }
    Ok(install_dry_run_action(&path, format))
}

fn parse_invoke_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut mode = None;
    let mut developer_trial = false;
    let mut signed = false;
    let mut module_id = None;
    let mut store_root = None;
    let mut challenge_issued_unix_seconds = None;
    let mut confirmation = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--developer-trial" if !developer_trial => developer_trial = true,
            "--developer-trial" => return Err(invoke_usage()),
            "--signed" if !signed => signed = true,
            "--signed" => return Err(invoke_usage()),
            "--dry-run" if mode.is_none() => mode = Some(false),
            "--dry-run" => return Err(invoke_usage()),
            "--apply" if mode.is_none() => mode = Some(true),
            "--apply" => return Err(invoke_usage()),
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            "--module-id" => set_option(&mut module_id, args, &mut index, "module ID")?,
            "--store-root" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(invoke_usage());
                };
                if store_root.is_some() {
                    return Err(
                        "module invocation store root was provided more than once".to_string()
                    );
                }
                store_root = Some(resolve_store_root(value)?);
                index += 1;
            }
            "--challenge-issued-unix-seconds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(invoke_usage());
                };
                if challenge_issued_unix_seconds.is_some() {
                    return Err(
                        "module invocation challenge time was provided more than once".to_string(),
                    );
                }
                challenge_issued_unix_seconds =
                    Some(value.parse::<u64>().map_err(|_| {
                        "challenge-issued-unix-seconds must be an integer".to_string()
                    })?);
                index += 1;
            }
            "--confirm" => set_option(&mut confirmation, args, &mut index, "confirmation phrase")?,
            _ => return Err(invoke_usage()),
        }
        index += 1;
    }
    if developer_trial == signed {
        return Err(
            "module invocation requires --developer-trial or --signed (exactly one is required)"
                .to_string(),
        );
    }
    let Some(module_id) = module_id else {
        return Err(format!(
            "module invocation requires --module-id <id>\n\n{}",
            invoke_usage()
        ));
    };
    let Some(store_root) = store_root else {
        return Err(format!(
            "module invocation requires --store-root <path>\n\n{}",
            invoke_usage()
        ));
    };
    let Some(apply) = mode else {
        return Err(invoke_usage());
    };
    let invocation_mode = if apply {
        let Some(issued) = challenge_issued_unix_seconds else {
            return Err(format!(
                "module invocation apply requires --challenge-issued-unix-seconds <seconds>\n\n{}",
                invoke_usage()
            ));
        };
        let Some(confirmation) = confirmation else {
            return Err(format!(
                "module invocation apply requires --confirm <exact-phrase>\n\n{}",
                invoke_usage()
            ));
        };
        crate::module_invoke::DeveloperInvocationMode::Apply {
            challenge_issued_unix_seconds: issued,
            confirmation,
        }
    } else {
        if challenge_issued_unix_seconds.is_some() || confirmation.is_some() {
            return Err(
                "module invocation dry-run does not accept apply confirmation arguments"
                    .to_string(),
            );
        }
        crate::module_invoke::DeveloperInvocationMode::DryRun
    };
    if signed {
        Ok(ModulesAction::InvokeSigned {
            request: crate::module_invoke::SignedInvocationRequest {
                module_id,
                store_root,
                mode: invocation_mode,
            },
            format,
        })
    } else {
        Ok(ModulesAction::InvokeDeveloperTrial {
            request: crate::module_invoke::DeveloperInvocationRequest {
                module_id,
                store_root,
                mode: invocation_mode,
            },
            format,
        })
    }
}

fn parse_lifecycle_execute_args(
    args: &[String],
    operation: LifecycleOperationLabel,
) -> Result<ModulesAction, String> {
    let mut format = OutputFormat::Text;
    let mut apply = None;
    let mut module_id = None;
    let mut recovery_id = None;
    let mut store_root = None;
    let mut package_path = None;
    let mut signature_path = None;
    let mut trusted_key_path = None;
    let mut challenge_issued_unix_seconds = None;
    let mut confirmation = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if apply.is_none() => apply = Some(false),
            "--apply" if apply.is_none() => apply = Some(true),
            "--dry-run" | "--apply" => return Err(lifecycle_execute_usage(operation)),
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            "--module-id" => set_option(&mut module_id, args, &mut index, "module ID")?,
            "--recovery-id" => set_option(&mut recovery_id, args, &mut index, "recovery ID")?,
            "--store-root" => set_option(&mut store_root, args, &mut index, "store root")?,
            "--package" => set_option(&mut package_path, args, &mut index, "package path")?,
            "--signature" => set_option(&mut signature_path, args, &mut index, "signature path")?,
            "--trusted-key" => {
                set_option(&mut trusted_key_path, args, &mut index, "trusted key path")?
            }
            "--challenge-issued-unix-seconds" => {
                let mut value = None;
                set_option(&mut value, args, &mut index, "challenge time")?;
                challenge_issued_unix_seconds =
                    Some(value.unwrap().parse::<u64>().map_err(|_| {
                        "challenge-issued-unix-seconds must be an integer".to_string()
                    })?);
            }
            "--confirm" => set_option(&mut confirmation, args, &mut index, "confirmation phrase")?,
            _ => return Err(lifecycle_execute_usage(operation)),
        }
        index += 1;
    }
    let Some(apply) = apply else {
        return Err(lifecycle_execute_usage(operation));
    };
    let Some(store_root) = store_root else {
        return Err(format!(
            "{} requires --store-root <path>\n\n{}",
            lifecycle_label(operation),
            lifecycle_execute_usage(operation)
        ));
    };
    let mode = if apply {
        let Some(issued) = challenge_issued_unix_seconds else {
            return Err(format!(
                "{} apply requires --challenge-issued-unix-seconds <seconds>\n\n{}",
                lifecycle_label(operation),
                lifecycle_execute_usage(operation)
            ));
        };
        let Some(confirmation) = confirmation else {
            return Err(format!(
                "{} apply requires --confirm <exact-phrase>\n\n{}",
                lifecycle_label(operation),
                lifecycle_execute_usage(operation)
            ));
        };
        LifecycleMode::Apply {
            challenge_issued_unix_seconds: issued,
            confirmation,
        }
    } else {
        if challenge_issued_unix_seconds.is_some() || confirmation.is_some() {
            return Err(format!(
                "{} dry-run does not accept apply confirmation arguments",
                lifecycle_label(operation)
            ));
        }
        LifecycleMode::DryRun
    };
    let lifecycle_operation = match operation {
        LifecycleOperationLabel::Enable => {
            if recovery_id.is_some()
                || package_path.is_some()
                || signature_path.is_some()
                || trusted_key_path.is_some()
            {
                return Err(lifecycle_execute_usage(operation));
            }
            LifecycleOperation::Enable
        }
        LifecycleOperationLabel::Disable => {
            if recovery_id.is_some()
                || package_path.is_some()
                || signature_path.is_some()
                || trusted_key_path.is_some()
            {
                return Err(lifecycle_execute_usage(operation));
            }
            LifecycleOperation::Disable
        }
        LifecycleOperationLabel::Update => {
            let Some(package_path) = package_path else {
                return Err(format!(
                    "module update requires --package <dir>\n\n{}",
                    lifecycle_execute_usage(operation)
                ));
            };
            let Some(signature_path) = signature_path else {
                return Err(format!(
                    "module update requires --signature <envelope.json>\n\n{}",
                    lifecycle_execute_usage(operation)
                ));
            };
            let Some(trusted_key_path) = trusted_key_path else {
                return Err(format!(
                    "module update requires --trusted-key <key.json>\n\n{}",
                    lifecycle_execute_usage(operation)
                ));
            };
            LifecycleOperation::Update {
                package_path: PathBuf::from(package_path),
                signature_path: PathBuf::from(signature_path),
                trusted_key_path: PathBuf::from(trusted_key_path),
            }
        }
        LifecycleOperationLabel::Uninstall => {
            if recovery_id.is_some()
                || package_path.is_some()
                || signature_path.is_some()
                || trusted_key_path.is_some()
            {
                return Err(lifecycle_execute_usage(operation));
            }
            LifecycleOperation::Uninstall
        }
        LifecycleOperationLabel::Recover => {
            if module_id.is_some()
                || package_path.is_some()
                || signature_path.is_some()
                || trusted_key_path.is_some()
            {
                return Err(lifecycle_execute_usage(operation));
            }
            if recovery_id.is_none() {
                return Err(format!(
                    "module recover requires --recovery-id <id>\n\n{}",
                    lifecycle_execute_usage(operation)
                ));
            }
            LifecycleOperation::Recover
        }
    };
    if !matches!(operation, LifecycleOperationLabel::Recover) && recovery_id.is_some() {
        return Err(lifecycle_execute_usage(operation));
    }
    if matches!(operation, LifecycleOperationLabel::Recover) && recovery_id.is_none() {
        return Err(lifecycle_execute_usage(operation));
    }
    Ok(ModulesAction::LifecycleExecute {
        request: LifecycleRequest {
            operation: lifecycle_operation,
            module_id,
            recovery_id,
            store_root: resolve_store_root(&store_root)?,
            mode,
        },
        format,
    })
}

fn parse_builtin_args(args: &[String]) -> Result<ModulesAction, String> {
    let Some(operation_name) = args.first().map(String::as_str) else {
        return Err(builtin_usage());
    };
    let operation = match operation_name {
        "install" => BuiltinOperation::Install,
        "enable" => BuiltinOperation::Enable,
        "disable" => BuiltinOperation::Disable,
        "update" => BuiltinOperation::Update,
        "uninstall" => BuiltinOperation::Uninstall,
        _ => return Err(builtin_usage()),
    };
    let mut format = OutputFormat::Text;
    let mut module_id = None;
    let mut store_root = None;
    let mut apply = None;
    let mut challenge_issued_unix_seconds = None;
    let mut confirmation = None;
    let mut index = 1usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if apply.is_none() => apply = Some(false),
            "--apply" if apply.is_none() => apply = Some(true),
            "--dry-run" | "--apply" => return Err(builtin_usage()),
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            "--module-id" => set_option(&mut module_id, args, &mut index, "module ID")?,
            "--store-root" => set_option(&mut store_root, args, &mut index, "store root")?,
            "--challenge-issued-unix-seconds" => {
                let mut value = None;
                set_option(&mut value, args, &mut index, "challenge time")?;
                challenge_issued_unix_seconds =
                    Some(value.unwrap().parse::<u64>().map_err(|_| {
                        "challenge-issued-unix-seconds must be an integer".to_string()
                    })?);
            }
            "--confirm" => set_option(&mut confirmation, args, &mut index, "confirmation phrase")?,
            _ => return Err(builtin_usage()),
        }
        index += 1;
    }
    let Some(module_id) = module_id else {
        return Err(format!(
            "module builtin {operation_name} requires --module-id <id>\n\n{}",
            builtin_usage()
        ));
    };
    let Some(apply) = apply else {
        return Err(builtin_usage());
    };
    let mode = if apply {
        let Some(issued) = challenge_issued_unix_seconds else {
            return Err(format!(
                "module builtin {operation_name} apply requires --challenge-issued-unix-seconds <seconds>\n\n{}",
                builtin_usage()
            ));
        };
        let Some(confirmation) = confirmation else {
            return Err(format!(
                "module builtin {operation_name} apply requires --confirm <exact-phrase>\n\n{}",
                builtin_usage()
            ));
        };
        BuiltinLifecycleMode::Apply {
            challenge_issued_unix_seconds: issued,
            confirmation,
        }
    } else {
        if challenge_issued_unix_seconds.is_some() || confirmation.is_some() {
            return Err(format!(
                "module builtin {operation_name} dry-run does not accept apply confirmation arguments"
            ));
        }
        BuiltinLifecycleMode::DryRun
    };
    Ok(ModulesAction::BuiltinLifecycle {
        request: BuiltinLifecycleRequest {
            operation,
            module_id,
            store_root: store_root
                .map(|root| resolve_store_root(&root))
                .transpose()?,
            mode,
        },
        format,
    })
}

fn parse_lifecycle_plan_args(args: &[String]) -> Result<ModulesAction, String> {
    let mut operation = None;
    let mut module_id = None;
    let mut from_state = None;
    let mut to_state = None;
    let mut from_version = None;
    let mut to_version = None;
    let mut transition_id = None;
    let mut format = OutputFormat::Text;
    let mut dry_run = false;
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--dry-run" => return Err(lifecycle_plan_usage()),
            "--module-id" => set_option(&mut module_id, args, &mut index, "module id")?,
            "--from-state" => set_option(&mut from_state, args, &mut index, "from state")?,
            "--to-state" => set_option(&mut to_state, args, &mut index, "to state")?,
            "--from-version" => set_option(&mut from_version, args, &mut index, "from version")?,
            "--to-version" => set_option(&mut to_version, args, &mut index, "to version")?,
            "--transition-id" => set_option(&mut transition_id, args, &mut index, "transition id")?,
            "--json" => format = OutputFormat::Json,
            "--format" => format = parse_format(args, &mut index)?,
            value if operation.is_none() => operation = Some(parse_operation(value)?),
            _ => return Err(lifecycle_plan_usage()),
        }
        index += 1;
    }

    if !dry_run {
        return Err(lifecycle_plan_usage());
    }
    let Some(operation) = operation else {
        return Err(lifecycle_plan_usage());
    };
    let Some(module_id) = module_id else {
        return Err(lifecycle_plan_usage());
    };
    let Some(from_state) = from_state.and_then(|value| parse_state(&value).ok()) else {
        return Err(lifecycle_plan_usage());
    };
    let Some(to_state) = to_state.and_then(|value| parse_state(&value).ok()) else {
        return Err(lifecycle_plan_usage());
    };

    Ok(ModulesAction::LifecyclePlan {
        operation,
        module_id: module_id.clone(),
        from_state,
        to_state,
        from_version,
        to_version,
        transition_id,
        format,
    })
}

fn set_option(
    slot: &mut Option<String>,
    args: &[String],
    index: &mut usize,
    label: &str,
) -> Result<(), String> {
    let Some(value) = args.get(*index + 1) else {
        return Err(format!("missing {label}\n\n{}", lifecycle_plan_usage()));
    };
    if slot.replace(value.clone()).is_some() {
        return Err(format!("{label} was provided more than once"));
    }
    *index += 1;
    Ok(())
}

fn parse_operation(value: &str) -> Result<ModuleLifecycleOperation, String> {
    match value {
        "install" => Ok(ModuleLifecycleOperation::Install),
        "activate" => Ok(ModuleLifecycleOperation::Activate),
        "invoke" => Ok(ModuleLifecycleOperation::Invoke),
        "deactivate" => Ok(ModuleLifecycleOperation::Deactivate),
        "repair" => Ok(ModuleLifecycleOperation::Repair),
        "migrate" => Ok(ModuleLifecycleOperation::Migrate),
        "upgrade" => Ok(ModuleLifecycleOperation::Upgrade),
        "uninstall" => Ok(ModuleLifecycleOperation::Uninstall),
        "recover" => Ok(ModuleLifecycleOperation::Recover),
        _ => Err(format!(
            "unsupported lifecycle operation '{value}'\n\n{}",
            lifecycle_plan_usage()
        )),
    }
}

fn parse_state(value: &str) -> Result<ModuleLifecycleState, String> {
    match value {
        "absent" => Ok(ModuleLifecycleState::Absent),
        "staged" => Ok(ModuleLifecycleState::Staged),
        "installed_inactive" => Ok(ModuleLifecycleState::InstalledInactive),
        "active" => Ok(ModuleLifecycleState::Active),
        "degraded" => Ok(ModuleLifecycleState::Degraded),
        "quarantined" => Ok(ModuleLifecycleState::Quarantined),
        _ => Err(format!("unsupported lifecycle state '{value}'")),
    }
}

fn parse_format(args: &[String], index: &mut usize) -> Result<OutputFormat, String> {
    let Some(value) = args.get(*index + 1).map(String::as_str) else {
        return Err(usage_error(args));
    };
    *index += 1;
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(usage_error(args)),
    }
}

fn install_dry_run_usage() -> String {
    format!(
        "module install planning is dry-run only\n\nUsage: {} modules install --dry-run <package-dir-or-manifest> [--format text|json]\n",
        brand::COMMAND
    )
}

fn install_usage() -> String {
    format!(
        "module installation is explicit and confirmation-gated\n\nUsage: {} modules install --dry-run <package-dir-or-manifest> [--format text|json]\n       {} modules install --signed --dry-run <package-dir-or-manifest> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> [--format text|json]\n       {} modules install --signed --apply <package-dir-or-manifest> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]\n       {} modules install --developer-trial --dry-run <package-dir-or-manifest> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> [--format text|json]\n       {} modules install --developer-trial --dry-run --developer-promote <package-dir-or-manifest> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> [--format text|json]\n       {} modules install --developer-trial --apply <package-dir-or-manifest> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]\n       {} modules install --developer-trial --apply --developer-promote <package-dir-or-manifest> --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]\n\nSigned first-party staging publishes only an installed_inactive record after exact package verification; activation, invocation, network fetch, and host mutation remain separately gated. The developer trial is test-key-only and never grants production execution authority.\n",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
    )
}

fn invoke_usage() -> String {
    format!(
        "module invocation is local and first-party-only\n\nUsage: {} modules invoke --signed --dry-run --module-id first-party.inventory --store-root <path> [--format text|json]\n       {} modules invoke --signed --apply --module-id first-party.inventory --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]\n       {} modules invoke --developer-trial --dry-run --module-id first-party.inventory --store-root <path> [--format text|json]\n       {} modules invoke --developer-trial --apply --module-id first-party.inventory --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]\n\nSigned invocation requires an active signed macOS inventory record and revalidates its complete immutable package before the bounded process host accepts a path-redacted read-only response. Developer-trial invocation is a promoted local fixture lane. Neither mode executes third-party code or provides a native sandbox.",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
    )
}

fn lifecycle_plan_usage() -> String {
    format!(
        "module lifecycle planning is dry-run only\n\nUsage: {} modules lifecycle-plan <install|activate|invoke|deactivate|repair|migrate|upgrade|uninstall> --dry-run --module-id <id> --from-state <state> --to-state <state> [--from-version <version>] [--to-version <version>] [--transition-id <id>] [--format text|json]\n\nStates: absent, staged, installed_inactive, active, degraded, quarantined\nSafety: this command creates a digest-bound, non-authorizing plan; it does not write, execute, activate, disable, or uninstall a module.\n",
        brand::COMMAND
    )
}

fn list_action(format: OutputFormat, from: Option<&String>) -> ModulesAction {
    ModulesAction::List {
        format,
        from: from.cloned(),
    }
}

fn validate_action(path: &str, format: OutputFormat) -> ModulesAction {
    ModulesAction::Validate {
        path: path.to_string(),
        format,
    }
}

fn install_dry_run_action(path: &str, format: OutputFormat) -> ModulesAction {
    ModulesAction::InstallDryRun {
        path: path.to_string(),
        format,
    }
}

fn render_modules(format: OutputFormat, from: Option<&str>) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => (ExitCode::Ok, modules_text_from(from), String::new()),
        OutputFormat::Json => match modules_json_from(from) {
            Ok(json) => (ExitCode::Ok, json, String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err),
        },
    }
}

fn render_module_status(
    format: OutputFormat,
    store_root: Option<PathBuf>,
) -> (ExitCode, String, String) {
    let report = module_status::module_status_report(&["modules status".to_string()], store_root);
    match format {
        OutputFormat::Text => (
            ExitCode::Ok,
            module_status::module_status_text(&report),
            String::new(),
        ),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn render_validation(format: OutputFormat, path: &str) -> (ExitCode, String, String) {
    let report = module_validation::load_manifest_file(Path::new(path));
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, validation_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn render_install_plan(format: OutputFormat, path: &str) -> (ExitCode, String, String) {
    let report = module_install_plan::plan_module_install_dry_run(Path::new(path));
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, install_plan_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn render_developer_stage(
    format: OutputFormat,
    request: &crate::module_stage::DeveloperStageRequest,
) -> (ExitCode, String, String) {
    let report = crate::module_stage::developer_stage_report(request);
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, developer_stage_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), error.to_string()),
        },
    }
}

fn render_signed_stage(
    format: OutputFormat,
    request: &crate::module_stage::SignedStageRequest,
) -> (ExitCode, String, String) {
    let report = crate::module_stage::signed_stage_report(request);
    let rendered = match format {
        OutputFormat::Text => signed_stage_text(&report),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => format!("{json}\n"),
            Err(error) => return (ExitCode::Usage, String::new(), error.to_string()),
        },
    };
    if report.valid {
        (ExitCode::Ok, rendered, String::new())
    } else {
        (ExitCode::Usage, rendered, String::new())
    }
}

fn render_builtin_lifecycle(
    format: OutputFormat,
    request: &BuiltinLifecycleRequest,
) -> (ExitCode, String, String) {
    let report = crate::builtin_modules::lifecycle_report(request);
    let rendered = match format {
        OutputFormat::Text => builtin_lifecycle_text(&report),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => format!("{json}\n"),
            Err(error) => return (ExitCode::Usage, String::new(), error.to_string()),
        },
    };
    if report.valid {
        (ExitCode::Ok, rendered, String::new())
    } else {
        (ExitCode::Usage, rendered, String::new())
    }
}

fn render_lifecycle_execution(
    format: OutputFormat,
    request: &LifecycleRequest,
) -> (ExitCode, String, String) {
    let report = crate::module_lifecycle_exec::lifecycle_report(request);
    let rendered = match format {
        OutputFormat::Text => lifecycle_execution_text(&report),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => format!("{json}\n"),
            Err(error) => return (ExitCode::Usage, String::new(), error.to_string()),
        },
    };
    if report.valid {
        (ExitCode::Ok, rendered, String::new())
    } else {
        (ExitCode::Usage, rendered, String::new())
    }
}

fn builtin_lifecycle_text(report: &crate::builtin_modules::BuiltinLifecycleReport) -> String {
    let mut out = format!("{} built-in module lifecycle\n\n", brand::TITLE);
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "operation: {}", report.operation);
    let _ = writeln!(out, "module_id: {}", report.module_id);
    let _ = writeln!(
        out,
        "display_name: {}",
        optional_value(report.display_name.as_deref())
    );
    let _ = writeln!(out, "state_before: {}", optional_value(report.state_before));
    let _ = writeln!(out, "state_after: {}", optional_value(report.state_after));
    let _ = writeln!(out, "valid: {}", report.valid);
    let _ = writeln!(out, "dry_run: {}", report.dry_run);
    let _ = writeln!(out, "writes_attempted: {}", report.writes_attempted);
    let _ = writeln!(out, "compiled_in: {}", report.compiled_in);
    let _ = writeln!(
        out,
        "product_execution_authorized: {}",
        report.product_execution_authorized
    );
    let _ = writeln!(
        out,
        "plan_sha256: {}",
        optional_value(report.plan_sha256.as_deref())
    );
    let _ = writeln!(out, "state_path: {}", report.state_path);
    for error in &report.errors {
        let _ = writeln!(out, "error: {error}");
    }
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    if let Some(challenge) = &report.challenge {
        let _ = writeln!(out, "confirmation_phrase: {}", challenge.expected_phrase);
        let _ = writeln!(
            out,
            "confirmation_expires_unix_seconds: {}",
            challenge.expires_unix_seconds
        );
    }
    out
}

fn lifecycle_execution_text(
    report: &crate::module_lifecycle_exec::LifecycleExecutionReport,
) -> String {
    let mut out = format!("{} module lifecycle\n\n", brand::TITLE);
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "operation: {:?}", report.operation);
    let _ = writeln!(
        out,
        "module_id: {}",
        report.module_id.as_deref().unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "from_state: {}",
        report.from_state.map(state_label).unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "to_state: {}",
        report.to_state.map(state_label).unwrap_or("none")
    );
    let _ = writeln!(
        out,
        "from_version: {}",
        optional_value(report.from_version.as_deref())
    );
    let _ = writeln!(
        out,
        "to_version: {}",
        optional_value(report.to_version.as_deref())
    );
    let _ = writeln!(out, "valid: {}", report.valid);
    let _ = writeln!(out, "dry_run: {}", report.dry_run);
    let _ = writeln!(out, "read_only: {}", report.read_only);
    let _ = writeln!(out, "writes_attempted: {}", report.writes_attempted);
    let _ = writeln!(
        out,
        "product_execution_authorized: {}",
        report.product_execution_authorized
    );
    let _ = writeln!(
        out,
        "plan_sha256: {}",
        optional_value(report.plan_sha256.as_deref())
    );
    let _ = writeln!(
        out,
        "transaction_id: {}",
        optional_value(report.transaction_id.as_deref())
    );
    let _ = writeln!(
        out,
        "receipt_path: {}",
        optional_value(report.receipt_path.as_deref())
    );
    let _ = writeln!(
        out,
        "recovery_id: {}",
        optional_value(report.recovery_id.as_deref())
    );
    if let Some(challenge) = &report.challenge {
        let _ = writeln!(
            out,
            "challenge_issued_unix_seconds: {}",
            challenge.issued_unix_seconds
        );
        let _ = writeln!(
            out,
            "challenge_expires_unix_seconds: {}",
            challenge.expires_unix_seconds
        );
        let _ = writeln!(out, "expected_phrase: {}", challenge.expected_phrase);
    }
    write_string_list(&mut out, "errors", &report.errors);
    write_string_list(&mut out, "warnings", &report.warnings);
    write_string_list(&mut out, "guidance", &report.guidance);
    out
}

fn write_string_list(out: &mut String, label: &str, values: &[String]) {
    let _ = writeln!(out, "{label}:");
    if values.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for value in values {
            let _ = writeln!(out, "  - {value}");
        }
    }
}

fn signed_stage_text(report: &crate::module_stage::DeveloperStageReport) -> String {
    let mut out = developer_stage_text(report);
    out = out.replace("developer module stage", "signed module install");
    out = out.replace("developer-only", "first-party release-key");
    out
}

fn developer_stage_text(report: &crate::module_stage::DeveloperStageReport) -> String {
    let mut out = format!("{} developer module stage\n\n", brand::TITLE);
    let _ = writeln!(
        out,
        "status: {}",
        if report.valid { "valid" } else { "blocked" }
    );
    let _ = writeln!(
        out,
        "mode: {}",
        if report.dry_run { "dry-run" } else { "apply" }
    );
    let _ = writeln!(
        out,
        "developer_only: {}",
        if report.developer_only { "yes" } else { "no" }
    );
    let _ = writeln!(
        out,
        "test_key_only: {}",
        if report.test_key_only { "yes" } else { "no" }
    );
    let _ = writeln!(
        out,
        "writes_attempted: {}",
        if report.writes_attempted { "yes" } else { "no" }
    );
    let _ = writeln!(
        out,
        "product_execution_authorized: {}",
        if report.product_execution_authorized {
            "yes"
        } else {
            "no"
        }
    );
    if let Some(id) = report.package_id.as_deref() {
        let _ = writeln!(
            out,
            "package: {}@{}",
            id,
            report.package_version.as_deref().unwrap_or("unknown")
        );
    }
    if let Some(destination) = report.destination_relative.as_deref() {
        let _ = writeln!(out, "destination: {destination}");
    }
    if let Some(plan_id) = report.plan_id.as_deref() {
        let _ = writeln!(out, "plan_id: {plan_id}");
    }
    if let Some(challenge) = &report.challenge {
        let _ = writeln!(
            out,
            "challenge_issued_unix_seconds: {}",
            challenge.issued_unix_seconds
        );
        let _ = writeln!(
            out,
            "challenge_expires_unix_seconds: {}",
            challenge.expires_unix_seconds
        );
        let _ = writeln!(out, "confirm_phrase: {}", challenge.expected_phrase);
    }
    let _ = writeln!(out, "files: {}", report.files.len());
    for file in &report.files {
        let _ = writeln!(
            out,
            "  - {} ({} bytes, {})",
            file.path, file.size_bytes, file.sha256
        );
    }
    if !report.errors.is_empty() {
        let _ = writeln!(out, "errors:");
        for error in &report.errors {
            let _ = writeln!(out, "  - {error}");
        }
    }
    if !report.warnings.is_empty() {
        let _ = writeln!(out, "warnings:");
        for warning in &report.warnings {
            let _ = writeln!(out, "  - {warning}");
        }
    }
    let _ = writeln!(out, "safety: {}", report.safety_note);
    out
}

fn render_developer_invocation(
    format: OutputFormat,
    request: &crate::module_invoke::DeveloperInvocationRequest,
) -> (ExitCode, String, String) {
    let report = crate::module_invoke::developer_invocation_report(request);
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, developer_invocation_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), error.to_string()),
        },
    }
}

fn render_signed_invocation(
    format: OutputFormat,
    request: &crate::module_invoke::SignedInvocationRequest,
) -> (ExitCode, String, String) {
    let report = crate::module_invoke::signed_invocation_report(request);
    let rendered = match format {
        OutputFormat::Text => developer_invocation_text(&report)
            .replace("developer module invocation", "signed module invocation"),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => format!("{json}\n"),
            Err(error) => return (ExitCode::Usage, String::new(), error.to_string()),
        },
    };
    if report.valid {
        (ExitCode::Ok, rendered, String::new())
    } else {
        (ExitCode::Usage, rendered, String::new())
    }
}

fn developer_invocation_text(report: &crate::module_invoke::DeveloperInvocationReport) -> String {
    let mut out = format!("{} developer module invocation\n\n", brand::TITLE);
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "module: {}", report.module_id);
    let _ = writeln!(
        out,
        "version: {}",
        report.module_version.as_deref().unwrap_or("unknown")
    );
    let _ = writeln!(out, "status: {:?}", report.status);
    let _ = writeln!(out, "dry_run: {}", report.dry_run);
    let _ = writeln!(out, "execution_attempted: {}", report.execution_attempted);
    let _ = writeln!(out, "writes_attempted: {}", report.writes_attempted);
    let _ = writeln!(
        out,
        "product_execution_authorized: {}",
        if report.product_execution_authorized {
            "yes"
        } else {
            "no"
        }
    );
    if let Some(plan_id) = &report.plan_id {
        let _ = writeln!(out, "plan_id: {plan_id}");
    }
    if let Some(plan_sha256) = &report.plan_sha256 {
        let _ = writeln!(out, "plan_sha256: {plan_sha256}");
    }
    if let Some(challenge) = &report.challenge {
        let _ = writeln!(
            out,
            "challenge_issued_unix_seconds: {}",
            challenge.issued_unix_seconds
        );
        let _ = writeln!(
            out,
            "challenge_expires_unix_seconds: {}",
            challenge.expires_unix_seconds
        );
        let _ = writeln!(out, "confirm_phrase: {}", challenge.expected_phrase);
    }
    if let Some(mechanism) = &report.binding_mechanism {
        let _ = writeln!(out, "binding_mechanism: {mechanism}");
    }
    if let Some(inventory) = &report.inventory {
        let _ = writeln!(out, "inventory_sources: {}", inventory.summary.source_count);
        let _ = writeln!(
            out,
            "inventory_path_entries: {}",
            inventory.summary.path_entry_count
        );
        let _ = writeln!(out, "inventory_tools: {}", inventory.summary.tool_count);
        let _ = writeln!(out, "inventory_apps: {}", inventory.summary.app_count);
        let _ = writeln!(
            out,
            "inventory_services: {}",
            inventory.summary.service_count
        );
        let _ = writeln!(
            out,
            "inventory_warnings: {}",
            inventory.summary.warning_count
        );
    }
    if !report.errors.is_empty() {
        let _ = writeln!(out, "errors:");
        for error in &report.errors {
            let _ = writeln!(out, "  - {error}");
        }
    }
    if !report.warnings.is_empty() {
        let _ = writeln!(out, "warnings:");
        for warning in &report.warnings {
            let _ = writeln!(out, "  - {warning}");
        }
    }
    let _ = writeln!(
        out,
        "safety: developer trial only; no activation or registry mutation"
    );
    out
}

fn render_lifecycle_plan(
    format: OutputFormat,
    request: LifecyclePlanRenderRequest,
) -> (ExitCode, String, String) {
    let LifecyclePlanRenderRequest {
        operation,
        module_id,
        from_state,
        to_state,
        from_version,
        to_version,
        transition_id,
    } = request;
    let transition_id = transition_id.unwrap_or_else(|| {
        format!(
            "module-lifecycle-{}-{}",
            operation_label(operation),
            module_id
        )
    });
    let plan = module_lifecycle_plan(
        transition_id,
        module_id,
        operation,
        from_state,
        to_state,
        from_version,
        to_version,
    );
    match plan {
        Ok(plan) => match format {
            OutputFormat::Text => (ExitCode::Ok, lifecycle_plan_text(&plan), String::new()),
            OutputFormat::Json => match serde_json::to_string_pretty(&plan) {
                Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
                Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
            },
        },
        Err(validation) => {
            let message = match format {
                OutputFormat::Text => {
                    let mut out = format!(
                        "{} module lifecycle plan\n\nstatus: invalid\n",
                        brand::TITLE
                    );
                    for error in validation.errors {
                        let _ = writeln!(out, "error: {error}");
                    }
                    out
                }
                OutputFormat::Json => {
                    serde_json::json!({
                        "schema_version": rz0_module_lifecycle::MODULE_LIFECYCLE_SCHEMA_VERSION,
                        "contract": rz0_module_lifecycle::MODULE_LIFECYCLE_CONTRACT,
                        "valid": false,
                        "errors": validation.errors,
                        "dry_run": true,
                        "writes_attempted": false,
                        "product_execution_authorized": false,
                    })
                    .to_string()
                        + "\n"
                }
            };
            (ExitCode::Usage, message, String::new())
        }
    }
}

fn lifecycle_plan_text(plan: &rz0_module_lifecycle::ModuleLifecyclePlan) -> String {
    let mut out = format!("{} module lifecycle plan\n\n", brand::TITLE);
    let _ = writeln!(out, "status: valid");
    let _ = writeln!(out, "contract: {}", plan.contract);
    let _ = writeln!(out, "transition_id: {}", plan.transition_id);
    let _ = writeln!(out, "module_id: {}", plan.module_id);
    let _ = writeln!(out, "operation: {}", operation_label(plan.operation));
    let _ = writeln!(out, "from_state: {}", state_label(plan.from_state));
    let _ = writeln!(out, "to_state: {}", state_label(plan.to_state));
    let _ = writeln!(
        out,
        "from_version: {}",
        optional_value(plan.from_version.as_deref())
    );
    let _ = writeln!(
        out,
        "to_version: {}",
        optional_value(plan.to_version.as_deref())
    );
    let _ = writeln!(
        out,
        "required_gates: {}",
        plan.required_gates
            .iter()
            .map(|gate| format!("{gate:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(out, "dry_run: {}", plan.dry_run);
    let _ = writeln!(out, "writes_attempted: {}", plan.writes_attempted);
    let _ = writeln!(out, "would_mutate: {}", plan.would_mutate);
    let _ = writeln!(out, "rollback_required: {}", plan.rollback_required);
    let _ = writeln!(
        out,
        "explicit_confirmation_required: {}",
        plan.explicit_confirmation_required
    );
    let _ = writeln!(
        out,
        "product_execution_authorized: {}",
        plan.product_execution_authorized
    );
    let _ = writeln!(out, "plan_sha256: {}", plan.plan_sha256);
    let _ = writeln!(
        out,
        "safety: dry-run plan only; no lifecycle action was executed"
    );
    out
}

fn optional_value(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn operation_label(operation: ModuleLifecycleOperation) -> &'static str {
    match operation {
        ModuleLifecycleOperation::Install => "install",
        ModuleLifecycleOperation::Activate => "activate",
        ModuleLifecycleOperation::Invoke => "invoke",
        ModuleLifecycleOperation::Deactivate => "deactivate",
        ModuleLifecycleOperation::Repair => "repair",
        ModuleLifecycleOperation::Migrate => "migrate",
        ModuleLifecycleOperation::Upgrade => "upgrade",
        ModuleLifecycleOperation::Uninstall => "uninstall",
        ModuleLifecycleOperation::Recover => "recover",
    }
}

fn state_label(state: ModuleLifecycleState) -> &'static str {
    match state {
        ModuleLifecycleState::Absent => "absent",
        ModuleLifecycleState::Staged => "staged",
        ModuleLifecycleState::InstalledInactive => "installed_inactive",
        ModuleLifecycleState::Active => "active",
        ModuleLifecycleState::Degraded => "degraded",
        ModuleLifecycleState::Quarantined => "quarantined",
    }
}

fn modules_text_from(from: Option<&str>) -> String {
    let report = registry_report(from);
    let mut out = format!("{} modules\n\n", brand::TITLE);
    write_core(&mut out, &report.core);
    write_builtins(&mut out, &report);
    write_installed(&mut out, &report);
    write_planned(&mut out, &report.planned_module_families);
    let _ = writeln!(
        out,
        "\nsafety: built-in capabilities are compiled into runtime.zero; optional package modules remain separately installed and confirmation-gated"
    );
    out
}

fn write_builtins(out: &mut String, report: &module_registry::ModuleRegistryReport) {
    let _ = writeln!(out, "\nbuilt-in capabilities:");
    for module in &report.built_in_capabilities {
        let state = if report
            .built_in_state
            .get(&module.id)
            .copied()
            .unwrap_or(true)
        {
            "enabled"
        } else {
            "disabled"
        };
        let _ = writeln!(out, "  {:<22} {state:<8} {}", module.id, module.summary);
    }
}

fn write_core(out: &mut String, modules: &[module_manifest::ModuleManifest]) {
    let _ = writeln!(out, "core foundation:");
    for module in modules {
        let _ = writeln!(out, "  {:<16} active   {}", module.id, module.summary);
    }
}

fn write_installed(out: &mut String, report: &module_registry::ModuleRegistryReport) {
    let _ = writeln!(out, "\ninstalled modules:");
    if report.installed_modules.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for module in &report.installed_modules {
            let _ = writeln!(out, "  {:<22} installed {}", module.id, module.summary);
        }
    }
    if !report.validation_reports.is_empty() {
        write_validation_summary(out, &report.validation_reports);
    }
}

fn write_validation_summary(
    out: &mut String,
    reports: &[module_validation::ManifestValidationReport],
) {
    let _ = writeln!(out, "\nvalidation:");
    for report in reports {
        let status = if report.valid { "valid" } else { "invalid" };
        let _ = writeln!(out, "  {:<8} {}", status, report.path);
    }
}

fn write_planned(out: &mut String, modules: &[module_manifest::ModuleManifest]) {
    let _ = writeln!(out, "\nplanned optional first-party package families:");
    for module in modules {
        let _ = writeln!(out, "  {:<22} planned  {}", module.id, module.summary);
    }
}

fn modules_json_from(from: Option<&str>) -> Result<String, String> {
    serde_json::to_string_pretty(&registry_report(from))
        .map(|json| format!("{json}\n"))
        .map_err(|err| format!("failed to render module registry JSON: {err}\n"))
}

fn registry_report(from: Option<&str>) -> module_registry::ModuleRegistryReport {
    match from {
        Some(path) => module_registry::ModuleRegistryReport::from_directory(Path::new(path)),
        None => module_registry::ModuleRegistryReport::empty_installed(),
    }
}

fn validation_text(report: &module_validation::ManifestValidationReport) -> String {
    let status = if report.valid { "valid" } else { "invalid" };
    let mut out = format!("{} module manifest validation\n\n", brand::TITLE);
    let _ = writeln!(out, "path: {}", report.path);
    let _ = writeln!(out, "status: {status}");
    for error in &report.errors {
        let _ = writeln!(out, "error: {error}");
    }
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    out
}

fn install_plan_text(report: &module_install_plan::ModuleInstallPlanReport) -> String {
    let status = if report.valid { "valid" } else { "invalid" };
    let mut out = format!("{} module install dry-run\n\n", brand::TITLE);
    let _ = writeln!(out, "input: {}", report.input_path);
    let _ = writeln!(out, "manifest: {}", report.manifest_path);
    let _ = writeln!(out, "package_root: {}", report.package_root);
    let _ = writeln!(out, "status: {status}");
    let _ = writeln!(out, "dry_run: true");
    let _ = writeln!(out, "writes_attempted: no");
    if let Some(target) = &report.proposed_module_dir {
        let _ = writeln!(out, "proposed_module_dir: {target}");
    }
    for action in &report.planned_actions {
        let _ = writeln!(
            out,
            "plan: {} -> {}",
            install_action_label(action.action),
            action.target
        );
    }
    for error in &report.errors {
        let _ = writeln!(out, "error: {error}");
    }
    for warning in &report.validation.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    let _ = writeln!(out, "safety: {}", report.safety_note);
    out
}

fn install_action_label(action: module_install_plan::PlannedInstallActionKind) -> &'static str {
    match action {
        module_install_plan::PlannedInstallActionKind::CreateModuleDirectory => {
            "create_module_directory"
        }
        module_install_plan::PlannedInstallActionKind::CopyPackageFile => "copy_package_file",
        module_install_plan::PlannedInstallActionKind::RecordInstalledManifest => {
            "record_installed_manifest"
        }
    }
}

fn usage_error(args: &[String]) -> String {
    format!(
        "unsupported modules option(s): {}\n\n{}",
        args.join(", "),
        modules_usage()
    )
}

fn modules_usage() -> String {
    format!(
        "Usage: {} modules [--from <dir>] [--format text|json]\n       {} modules status [--store-root <path>] [--format text|json]\n       {} modules builtin <install|enable|disable|update|uninstall> --module-id <id> --dry-run|--apply [--store-root <path>] [--format text|json]\n       {} modules enable|disable|update|uninstall --module-id first-party.inventory --store-root <path> --dry-run|--apply ...\n       {} modules recover --recovery-id <id> --store-root <path> --dry-run|--apply ...\n       {} modules validate <manifest.json> [--format text|json]\n       {} modules install --dry-run <package-dir-or-manifest> [--format text|json]\n       {} modules install --developer-trial --dry-run <package-dir-or-manifest> [--developer-promote] --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> [--format text|json]\n       {} modules install --developer-trial --apply <package-dir-or-manifest> [--developer-promote] --signature <envelope.json> --trusted-test-key <key.json> --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]\n       {} modules invoke --developer-trial --dry-run --module-id first-party.inventory --store-root <path> [--format text|json]\n       {} modules invoke --developer-trial --apply --module-id first-party.inventory --store-root <path> --challenge-issued-unix-seconds <seconds> --confirm <exact-phrase> [--format text|json]\n       {} modules lifecycle-plan <operation> --dry-run --module-id <id> --from-state <state> --to-state <state> [--from-version <version>] [--to-version <version>] [--format text|json]\n       {} modules trust verify --manifest <manifest.json> --signature <envelope.json> --trusted-test-key <key.json> [--format text|json]\n\nSafety: built-in lifecycle changes only runtime.zero availability state and never deletes compiled bytes; signed optional module lifecycle changes only an explicitly initialized private store; all apply operations rebuild an exact plan and require a short-lived confirmation; optional modules are not executed or fetched by default; third-party modules are not fetched or executed.\n",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND
    )
}

fn lifecycle_label(operation: LifecycleOperationLabel) -> &'static str {
    match operation {
        LifecycleOperationLabel::Enable => "module enable",
        LifecycleOperationLabel::Disable => "module disable",
        LifecycleOperationLabel::Update => "module update",
        LifecycleOperationLabel::Uninstall => "module uninstall",
        LifecycleOperationLabel::Recover => "module recover",
    }
}

fn builtin_usage() -> String {
    format!(
        "built-in lifecycle changes local availability only\n\nUsage: {} modules builtin <install|enable|disable|update|uninstall> --module-id <id> --dry-run|--apply [--store-root <path>] [--challenge-issued-unix-seconds <seconds> --confirm <exact-phrase>] [--format text|json]\n\nBuilt-in bytes ship inside runtime.zero. Install and uninstall change availability state; update is provided by upgrading runtime.zero; every apply requires the exact challenge returned by dry-run.",
        brand::COMMAND
    )
}

fn lifecycle_execute_usage(operation: LifecycleOperationLabel) -> String {
    match operation {
        LifecycleOperationLabel::Enable => format!(
            "Usage: {} modules enable --module-id first-party.inventory --store-root <path> --dry-run|--apply [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]",
            brand::COMMAND
        ),
        LifecycleOperationLabel::Disable => format!(
            "Usage: {} modules disable --module-id first-party.inventory --store-root <path> --dry-run|--apply [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]",
            brand::COMMAND
        ),
        LifecycleOperationLabel::Update => format!(
            "Usage: {} modules update --module-id first-party.inventory --package <dir> --signature <envelope.json> --trusted-key <key.json> --store-root <path> --dry-run|--apply [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]",
            brand::COMMAND
        ),
        LifecycleOperationLabel::Uninstall => format!(
            "Usage: {} modules uninstall --module-id first-party.inventory --store-root <path> --dry-run|--apply [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]",
            brand::COMMAND
        ),
        LifecycleOperationLabel::Recover => format!(
            "Usage: {} modules recover --recovery-id <id> --store-root <path> --dry-run|--apply [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]",
            brand::COMMAND
        ),
    }
}

fn status_usage() -> String {
    format!(
        "module status is read-only\n\nUsage: {} modules status [--store-root <path>] [--format text|json]\n\nIt reports installed, degraded, and absent module state without activating, invoking, installing, or repairing modules.\n",
        brand::COMMAND
    )
}
