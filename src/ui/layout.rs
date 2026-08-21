use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTier {
    VerySmall,
    Compact,
    Standard,
    Wide,
}

impl LayoutTier {
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

    pub const fn minimum_size(self) -> &'static str {
        match self {
            Self::VerySmall => "<50x12",
            Self::Compact => "50x12",
            Self::Standard => "72x20",
            Self::Wide => "110x24",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutPlan {
    pub tier: LayoutTier,
    pub header: Rect,
    pub routes: Rect,
    pub primary: Rect,
    pub detail: Rect,
    pub status: Rect,
    pub keys: Rect,
    pub overlay: Rect,
}

impl LayoutPlan {
    pub fn for_area(area: Rect) -> Self {
        let tier = LayoutTier::from_size(area.width, area.height);
        let header = take_top(area, 2);
        let after_header = below(area, 2);
        let routes = take_top(after_header, 2);
        let body = below(after_header, 2);
        let keys = take_bottom(body, 1);
        let status = take_bottom(below(body, 1), 1);
        let content = above(body, 2);
        let (primary, detail) = match tier {
            LayoutTier::VerySmall => (content, Rect::default()),
            LayoutTier::Compact | LayoutTier::Standard => {
                let detail_height =
                    content
                        .height
                        .min(if tier == LayoutTier::Compact { 7 } else { 9 });
                (
                    Rect::new(
                        content.x,
                        content.y,
                        content.width,
                        content.height.saturating_sub(detail_height + 1),
                    ),
                    Rect::new(
                        content.x,
                        content.bottom().saturating_sub(detail_height),
                        content.width,
                        detail_height,
                    ),
                )
            }
            LayoutTier::Wide => {
                let detail_width = content.width.min(44);
                (
                    Rect::new(
                        content.x,
                        content.y,
                        content.width.saturating_sub(detail_width + 1),
                        content.height,
                    ),
                    Rect::new(
                        content.right().saturating_sub(detail_width),
                        content.y,
                        detail_width,
                        content.height,
                    ),
                )
            }
        };
        let overlay = centered(
            area,
            area.width.saturating_sub(6),
            area.height.saturating_sub(4),
        );
        Self {
            tier,
            header,
            routes,
            primary,
            detail,
            status,
            keys,
            overlay,
        }
    }
}

fn take_top(area: Rect, height: u16) -> Rect {
    Rect::new(area.x, area.y, area.width, area.height.min(height))
}

fn below(area: Rect, height: u16) -> Rect {
    Rect::new(
        area.x,
        area.y.saturating_add(height),
        area.width,
        area.height.saturating_sub(height),
    )
}

fn take_bottom(area: Rect, height: u16) -> Rect {
    Rect::new(
        area.x,
        area.bottom().saturating_sub(height),
        area.width,
        area.height.min(height),
    )
}

fn above(area: Rect, height: u16) -> Rect {
    Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(height),
    )
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_tiers_match_the_rfc_floor_and_review_sizes() {
        assert_eq!(LayoutTier::from_size(42, 10), LayoutTier::VerySmall);
        assert_eq!(LayoutTier::from_size(58, 16), LayoutTier::Compact);
        assert_eq!(LayoutTier::from_size(80, 24), LayoutTier::Standard);
        assert_eq!(LayoutTier::from_size(118, 30), LayoutTier::Wide);
    }

    #[test]
    fn named_regions_stay_inside_the_terminal() {
        for (width, height) in [(42, 10), (58, 16), (80, 24), (118, 30), (160, 50)] {
            let area = Rect::new(0, 0, width, height);
            let plan = LayoutPlan::for_area(area);
            for region in [
                plan.header,
                plan.routes,
                plan.primary,
                plan.detail,
                plan.status,
                plan.keys,
                plan.overlay,
            ] {
                assert!(region.x >= area.x);
                assert!(region.y >= area.y);
                assert!(region.right() <= area.right());
                assert!(region.bottom() <= area.bottom());
            }
        }
    }
}
