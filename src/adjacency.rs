use crate::geometry::{Half, Rect};

/// Find the monitor adjacent to `current` in the direction implied by `direction`.
/// Returns the index into `monitors`, or None if no neighbor.
///
/// Rules:
/// - Left: another monitor whose `right` edge equals `current.left`, with vertical overlap > 0.
/// - Right: symmetric with `left == current.right`.
/// - Top: another monitor whose `bottom` edge equals `current.top`, with horizontal overlap > 0.
/// - Bottom: symmetric with `top == current.bottom`.
/// - On multiple candidates, pick the one with greatest perpendicular-axis overlap.
pub fn find_adjacent_monitor(monitors: &[Rect], current: Rect, direction: Half) -> Option<usize> {
    let mut best_index: Option<usize> = None;
    let mut best_overlap: i32 = 0;

    for (index, candidate) in monitors.iter().enumerate() {
        if *candidate == current {
            continue;
        }
        let (edge_match, overlap) = match direction {
            Half::Left => (
                candidate.x + candidate.width == current.x,
                vertical_overlap(*candidate, current),
            ),
            Half::Right => (
                candidate.x == current.x + current.width,
                vertical_overlap(*candidate, current),
            ),
            Half::Top => (
                candidate.y + candidate.height == current.y,
                horizontal_overlap(*candidate, current),
            ),
            Half::Bottom => (
                candidate.y == current.y + current.height,
                horizontal_overlap(*candidate, current),
            ),
        };

        if edge_match && overlap > 0 && overlap > best_overlap {
            best_index = Some(index);
            best_overlap = overlap;
        }
    }

    best_index
}

fn vertical_overlap(a: Rect, b: Rect) -> i32 {
    let top = a.y.max(b.y);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (bottom - top).max(0)
}

fn horizontal_overlap(a: Rect, b: Rect) -> i32 {
    let left = a.x.max(b.x);
    let right = (a.x + a.width).min(b.x + b.width);
    (right - left).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_left_monitor_in_horizontal_setup() {
        let monitor_left = Rect::new(-1920, 0, 1920, 1080);
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_left, monitor_main];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn finds_right_monitor() {
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitor_right = Rect::new(1920, 0, 1920, 1080);
        let monitors = vec![monitor_main, monitor_right];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Right);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn no_left_neighbor_returns_none() {
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitor_right = Rect::new(1920, 0, 1920, 1080);
        let monitors = vec![monitor_main, monitor_right];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, None);
    }

    #[test]
    fn picks_greatest_overlap_among_multiple_left_candidates() {
        // Two stacked monitors on the left, both share the right-edge x.
        // Top one barely overlaps current vertically; bottom one fully overlaps.
        let monitor_top_left = Rect::new(-1920, -1000, 1920, 1080);   // overlaps y=0..80
        let monitor_bottom_left = Rect::new(-1920, 0, 1920, 1080);    // overlaps y=0..1080
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_top_left, monitor_bottom_left, monitor_main];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn finds_top_monitor() {
        let monitor_top = Rect::new(0, -1080, 1920, 1080);
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_top, monitor_main];

        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Top);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn touching_corner_only_does_not_count() {
        // Top-left monitor touches main only at the corner — no overlap on either axis.
        let monitor_corner = Rect::new(-1920, -1080, 1920, 1080);
        let monitor_main = Rect::new(0, 0, 1920, 1080);
        let monitors = vec![monitor_corner, monitor_main];

        // Looking left: edge matches (right = -1920+1920 = 0 = current.x) but vertical overlap is 0.
        let result = find_adjacent_monitor(&monitors, monitor_main, Half::Left);
        assert_eq!(result, None);
    }
}
