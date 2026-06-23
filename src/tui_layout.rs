#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiLayoutTier {
    VerySmall,
    Compact,
    Standard,
    Wide,
}

impl TuiLayoutTier {
    pub const fn from_size(width: u16, height: u16) -> Self {
        if width < 50 || height < 12 {
            Self::VerySmall
        } else if width < 72 || height < 20 {
            Self::Compact
        } else if width >= 110 && height >= 24 {
            Self::Wide
        } else {
            Self::Standard
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::VerySmall => "very-small",
            Self::Compact => "compact",
            Self::Standard => "standard",
            Self::Wide => "wide",
        }
    }

    pub const fn minimum_size(self) -> &'static str {
        match self {
            Self::VerySmall => "<50x12",
            Self::Compact => "50x12",
            Self::Standard => "72x20",
            Self::Wide => "110x24",
        }
    }

    pub const fn uses_full_dashboard(self) -> bool {
        matches!(self, Self::Standard | Self::Wide)
    }
}
