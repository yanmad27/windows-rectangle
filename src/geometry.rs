#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Halve(Half),
    Quarter(Corner),
    Maximize,
    Center,
    RestoreOriginal,
}

/// Compute target rect for a given action within a work area.
/// `current` is needed for Center (preserve size) and is otherwise unused.
pub fn compute_target_rect(work_area: Rect, action: Action, current: Rect) -> Option<Rect> {
    match action {
        Action::Halve(Half::Left) => Some(Rect::new(
            work_area.x,
            work_area.y,
            work_area.width / 2,
            work_area.height,
        )),
        Action::Halve(Half::Right) => Some(Rect::new(
            work_area.x + work_area.width / 2,
            work_area.y,
            work_area.width - work_area.width / 2,
            work_area.height,
        )),
        Action::Halve(Half::Top) => Some(Rect::new(
            work_area.x,
            work_area.y,
            work_area.width,
            work_area.height / 2,
        )),
        Action::Halve(Half::Bottom) => Some(Rect::new(
            work_area.x,
            work_area.y + work_area.height / 2,
            work_area.width,
            work_area.height - work_area.height / 2,
        )),
        Action::Quarter(Corner::TopLeft) => Some(Rect::new(
            work_area.x,
            work_area.y,
            work_area.width / 2,
            work_area.height / 2,
        )),
        Action::Quarter(Corner::TopRight) => Some(Rect::new(
            work_area.x + work_area.width / 2,
            work_area.y,
            work_area.width - work_area.width / 2,
            work_area.height / 2,
        )),
        Action::Quarter(Corner::BottomLeft) => Some(Rect::new(
            work_area.x,
            work_area.y + work_area.height / 2,
            work_area.width / 2,
            work_area.height - work_area.height / 2,
        )),
        Action::Quarter(Corner::BottomRight) => Some(Rect::new(
            work_area.x + work_area.width / 2,
            work_area.y + work_area.height / 2,
            work_area.width - work_area.width / 2,
            work_area.height - work_area.height / 2,
        )),
        Action::Center => Some(Rect::new(
            work_area.x + (work_area.width - current.width) / 2,
            work_area.y + (work_area.height - current.height) / 2,
            current.width,
            current.height,
        )),
        // Maximize and RestoreOriginal are not pure-geometry — they need IsZoomed / restore stack.
        Action::Maximize | Action::RestoreOriginal => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work_area_1920_1080() -> Rect {
        Rect::new(0, 0, 1920, 1040) // taskbar takes 40px
    }

    #[test]
    fn halve_left_takes_left_half() {
        let work_area = work_area_1920_1080();
        let result = compute_target_rect(work_area, Action::Halve(Half::Left), work_area).unwrap();
        assert_eq!(result, Rect::new(0, 0, 960, 1040));
    }

    #[test]
    fn halve_right_takes_right_half_with_remainder() {
        let work_area = Rect::new(0, 0, 1921, 1080); // odd width
        let result = compute_target_rect(work_area, Action::Halve(Half::Right), work_area).unwrap();
        assert_eq!(result, Rect::new(960, 0, 961, 1080));
        assert_eq!(result.x + result.width, 1921, "must reach right edge");
    }

    #[test]
    fn halve_bottom_starts_at_midpoint() {
        let work_area = Rect::new(0, 0, 1920, 1080);
        let result = compute_target_rect(work_area, Action::Halve(Half::Bottom), work_area).unwrap();
        assert_eq!(result, Rect::new(0, 540, 1920, 540));
    }

    #[test]
    fn quarter_top_left() {
        let work_area = work_area_1920_1080();
        let result = compute_target_rect(work_area, Action::Quarter(Corner::TopLeft), work_area).unwrap();
        assert_eq!(result, Rect::new(0, 0, 960, 520));
    }

    #[test]
    fn quarter_bottom_right_reaches_corners() {
        let work_area = Rect::new(0, 0, 1921, 1081); // odd both
        let result = compute_target_rect(work_area, Action::Quarter(Corner::BottomRight), work_area).unwrap();
        assert_eq!(result.x + result.width, 1921);
        assert_eq!(result.y + result.height, 1081);
    }

    #[test]
    fn center_preserves_size() {
        let work_area = Rect::new(0, 0, 1920, 1080);
        let current = Rect::new(100, 100, 800, 600);
        let result = compute_target_rect(work_area, Action::Center, current).unwrap();
        assert_eq!(result, Rect::new(560, 240, 800, 600));
    }

    #[test]
    fn center_on_offset_work_area() {
        // Second monitor at x=1920
        let work_area = Rect::new(1920, 0, 1920, 1080);
        let current = Rect::new(0, 0, 800, 600);
        let result = compute_target_rect(work_area, Action::Center, current).unwrap();
        assert_eq!(result, Rect::new(1920 + 560, 240, 800, 600));
    }

    #[test]
    fn maximize_returns_none_pure_geometry() {
        let work_area = Rect::new(0, 0, 1920, 1080);
        assert!(compute_target_rect(work_area, Action::Maximize, work_area).is_none());
    }
}
