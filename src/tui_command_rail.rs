#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TuiCommandPreview {
    pub label: &'static str,
    pub command: &'static str,
    pub preview: &'static str,
}

pub(crate) const COMMANDS: [TuiCommandPreview; 6] = [
    TuiCommandPreview {
        label: "doctor",
        command: "rz0 doctor",
        preview: "check the local environment and version",
    },
    TuiCommandPreview {
        label: "store status",
        command: "rz0 store status",
        preview: "show local transaction, registry, and receipt state",
    },
    TuiCommandPreview {
        label: "dashboard json",
        command: "rz0 --json",
        preview: "export the dashboard as JSON",
    },
    TuiCommandPreview {
        label: "software",
        command: "rz0 apps",
        preview: "list installed applications and packages",
    },
    TuiCommandPreview {
        label: "uninstall",
        command: "rz0 uninstall plan <id>",
        preview: "create an uninstall review for one installed item",
    },
    TuiCommandPreview {
        label: "install dry-run",
        command: "rz0 modules install --dry-run <package>",
        preview: "check a module package before installation",
    },
];

pub(crate) fn selected_command(index: usize) -> TuiCommandPreview {
    COMMANDS[index.min(COMMANDS.len().saturating_sub(1))]
}
