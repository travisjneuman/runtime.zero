pub const SURFACE_BG: &str = "#0A141D";
pub const PANEL_BG: &str = "#101C27";
pub const BRAND_ACCENT: &str = "#C6A15B";
pub const TEXT_PRIMARY: &str = "#E6E0D2";
pub const TEXT_MUTED: &str = "#8996A0";
pub const SAFE: &str = "#8FA88C";
pub const DRY_RUN: &str = "#AFA0D6";
pub const INFO: &str = "#7F9CAF";

pub const LABEL_OK: &str = "[OK]";
pub const LABEL_INFO: &str = "[INFO]";
pub const LABEL_PLAN: &str = "[PLAN]";
pub const LABEL_DRY_RUN: &str = "[DRY-RUN]";
pub const LABEL_WARN: &str = "[WARN]";
pub const LABEL_BLOCKED: &str = "[BLOCKED]";
pub const LABEL_SKIP: &str = "[SKIP]";

pub const BORDER_HORIZONTAL: &str = "─";
pub const BORDER_VERTICAL: &str = "│";
pub const BORDER_TOP_LEFT: &str = "┌";
pub const BORDER_TOP_RIGHT: &str = "┐";
pub const BORDER_BOTTOM_LEFT: &str = "└";
pub const BORDER_BOTTOM_RIGHT: &str = "┘";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiTone {
    Accent,
    Info,
    Safe,
    DryRun,
    Warn,
    Muted,
}

pub fn ansi(tone: TuiTone) -> &'static str {
    match tone {
        TuiTone::Accent => "\x1b[38;5;179m",
        TuiTone::Info => "\x1b[38;5;110m",
        TuiTone::Safe => "\x1b[38;5;108m",
        TuiTone::DryRun => "\x1b[38;5;147m",
        TuiTone::Warn => "\x1b[38;5;179m",
        TuiTone::Muted => "\x1b[38;5;245m",
    }
}

pub const ANSI_RESET: &str = "\x1b[0m";
