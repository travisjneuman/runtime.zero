use ratatui::style::{Color, Modifier, Style};

use super::model::RecordStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Accent,
    Info,
    Safe,
    DryRun,
    Warn,
    Danger,
    Muted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub color: bool,
}

impl Theme {
    pub const fn new(color: bool) -> Self {
        Self { color }
    }

    pub fn tone(self, tone: Tone) -> Style {
        if !self.color {
            return Style::default();
        }
        Style::default().fg(match tone {
            Tone::Accent => Color::Rgb(216, 187, 115),
            Tone::Info => Color::Rgb(127, 156, 175),
            Tone::Safe => Color::Rgb(143, 168, 140),
            Tone::DryRun => Color::Rgb(175, 160, 214),
            Tone::Warn => Color::Rgb(209, 155, 82),
            Tone::Danger => Color::Rgb(196, 90, 80),
            Tone::Muted => Color::Rgb(137, 150, 160),
        })
    }

    pub fn selected(self) -> Style {
        if self.color {
            Style::default()
                .fg(Color::Rgb(230, 224, 210))
                .bg(Color::Rgb(38, 54, 68))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
        }
    }

    pub fn heading(self) -> Style {
        self.tone(Tone::Accent).add_modifier(Modifier::BOLD)
    }

    pub fn status(self, status: RecordStatus) -> Style {
        self.tone(match status {
            RecordStatus::Ok => Tone::Safe,
            RecordStatus::Info | RecordStatus::Observed => Tone::Info,
            RecordStatus::Plan => Tone::Accent,
            RecordStatus::DryRun => Tone::DryRun,
            RecordStatus::Warn => Tone::Warn,
            RecordStatus::Blocked | RecordStatus::Error => Tone::Danger,
            RecordStatus::Muted => Tone::Muted,
        })
    }
}
