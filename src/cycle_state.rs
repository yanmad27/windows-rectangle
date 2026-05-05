use crate::geometry::Action;

/// Identifies a window for cycle-state purposes. The Win32 layer passes
/// HWND.0 as i64 here; tests use any unique integer.
pub type WindowId = i64;

/// Identifies a monitor for cycle-state purposes. The Win32 layer passes
/// HMONITOR.0 as i64; tests use any unique integer.
pub type MonitorId = i64;

#[derive(Debug, Default)]
pub struct CycleState {
    last: Option<(WindowId, Action, MonitorId)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CycleDecision {
    /// First press, or different window/action: apply on the current monitor.
    ApplyHere,
    /// Repeat press on the same window+action+monitor: move to the adjacent monitor.
    MoveToAdjacent,
}

impl CycleState {
    pub fn new() -> Self {
        Self { last: None }
    }

    /// Decide what to do for an incoming action on `window` currently on `monitor`.
    /// Only `Halve` actions cycle; everything else returns `ApplyHere`.
    pub fn decide(&self, window: WindowId, action: Action, monitor: MonitorId) -> CycleDecision {
        if !matches!(action, Action::Halve(_)) {
            return CycleDecision::ApplyHere;
        }
        match self.last {
            Some((prev_window, prev_action, prev_monitor))
                if prev_window == window
                    && prev_action == action
                    && prev_monitor == monitor =>
            {
                CycleDecision::MoveToAdjacent
            }
            _ => CycleDecision::ApplyHere,
        }
    }

    /// Record that `action` was just applied to `window` on `monitor`.
    pub fn record(&mut self, window: WindowId, action: Action, monitor: MonitorId) {
        if matches!(action, Action::Halve(_)) {
            self.last = Some((window, action, monitor));
        } else {
            self.last = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Half;

    const WINDOW_A: WindowId = 100;
    const WINDOW_B: WindowId = 200;
    const MONITOR_MAIN: MonitorId = 1;
    const MONITOR_LEFT: MonitorId = 2;

    #[test]
    fn first_halve_press_applies_here() {
        let state = CycleState::new();
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn repeat_same_action_same_monitor_moves_to_adjacent() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::MoveToAdjacent
        );
    }

    #[test]
    fn different_window_resets_cycle() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_B, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn different_direction_resets_cycle() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Right), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn quarter_action_does_not_cycle() {
        use crate::geometry::Corner;
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Quarter(Corner::TopLeft), MONITOR_MAIN);
        assert_eq!(
            state.decide(WINDOW_A, Action::Quarter(Corner::TopLeft), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn non_halve_action_clears_cycle_record() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        state.record(WINDOW_A, Action::Center, MONITOR_MAIN);
        // Now even repeating the prior halve should apply here, not move.
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN),
            CycleDecision::ApplyHere
        );
    }

    #[test]
    fn moving_to_adjacent_monitor_then_repeating_continues_cycle() {
        let mut state = CycleState::new();
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_MAIN);
        // After dispatcher acts on adjacent and records:
        state.record(WINDOW_A, Action::Halve(Half::Left), MONITOR_LEFT);
        // Next press should request another move (to monitor further left).
        assert_eq!(
            state.decide(WINDOW_A, Action::Halve(Half::Left), MONITOR_LEFT),
            CycleDecision::MoveToAdjacent
        );
    }
}
