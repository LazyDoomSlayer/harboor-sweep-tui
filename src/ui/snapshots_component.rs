use crate::ui::theme::TableColors;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    prelude::Style,
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Clear, Paragraph, Wrap},
};

use crate::util::popup_area;

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
    pub fn render(&self, frame: &mut Frame, area: Rect, colors: &TableColors) {
        if !self.display {
            return;
        }

        let block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::new().fg(colors.footer_border_color))
            .bg(colors.buffer_bg)
            .title("Snapshotting");

        let area = popup_area(area, 4, 5);
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
    }
}
