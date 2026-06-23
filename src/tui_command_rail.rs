#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TuiCommandPreview {
    pub label: &'static str,
    pub command: &'static str,
    pub preview: &'static str,
}

pub(crate) const COMMANDS: [TuiCommandPreview; 4] = [
    TuiCommandPreview {
        label: "doctor",
        command: "rz0 doctor",
        preview: "read-only environment and safety posture check",
    },
    TuiCommandPreview {
        label: "store status",
        command: "rz0 store status",
        preview: "inspect local store/registry/receipt state without writes",
    },
    TuiCommandPreview {
        label: "dashboard json",
        command: "rz0 --json",
        preview: "emit stable foundation_dashboard JSON for automation",
    },
    TuiCommandPreview {
        label: "install dry-run",
        command: "rz0 modules install --dry-run <package>",
        preview: "validate a local module package plan without installing it",
    },
];

pub(crate) fn selected_command(index: usize) -> TuiCommandPreview {
    COMMANDS[index.min(COMMANDS.len().saturating_sub(1))]
}
