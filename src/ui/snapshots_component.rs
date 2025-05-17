use crate::ui::theme::TableColors;

use ratatui::{Frame, layout::Rect};

/// A component that handles the snapshots
#[derive(Debug)]
pub struct SnapshotsComponent {
    /// Whether the popup is displayed
    pub display: bool,
}

impl Default for SnapshotsComponent {
    fn default() -> Self {
        Self { display: false }
    }
}

impl SnapshotsComponent {
    /// Toggle display on/off, clear selection when opening
    pub fn toggle(&mut self) {
        self.display = !self.display;
    }

    /// Renders the popup
    pub fn render(&self, _frame: &mut Frame, _area: Rect, _colors: &TableColors) {
        if !self.display {
            return;
        }
    }
}
