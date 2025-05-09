use crate::model::PortInfo;
use crate::ui::theme::TableColors;

use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Modifier, Style, Stylize},
    widgets::HighlightSpacing,
    widgets::{Cell, Row, Scrollbar, ScrollbarState, Table, TableState},
};

/// A component that handles rendering a scrollable table of PortInfo

#[derive(Debug)]
pub struct ProcessTableComponent {
    /// Filtered processes to display
    pub items: Vec<PortInfo>,
    /// Table selection state
    pub state: TableState,
    /// Scrollbar state
    pub scroll: ScrollbarState,
    /// Number of visible rows (set during render)
    pub visible_rows: usize,
    /// Pre-computed column width constraints
    pub column_widths: (u16, u16, u16, u16, u16),
}

impl Default for ProcessTableComponent {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            state: TableState::default(),
            scroll: ScrollbarState::new(1),
            visible_rows: 0,
            // Default width hints: Port, PID, Name, Path, Listener
            column_widths: (5, 5, 30, 55, 5),
        }
    }
}

impl ProcessTableComponent {
    /// Replace current items and update scrollbar length
    pub fn set_items(&mut self, items: Vec<PortInfo>) {
        self.items = items;
        let content_len = self.items.len() * crate::ITEM_HEIGHT as usize;
        self.scroll = self.scroll.content_length(content_len);
    }

    /// Move selection down by one row
    pub fn next_row(&mut self) {
        let len = self.items.len();
        let idx = match self.state.selected() {
            Some(i) if i + 1 < len => i + 1,
            _ if len > 0 => 0,
            _ => return,
        };
        self.state.select(Some(idx));
        self.scroll = self.scroll.position(idx * crate::ITEM_HEIGHT as usize);
    }

    /// Move selection up by one row
    pub fn previous_row(&mut self) {
        let len = self.items.len();
        let idx = match self.state.selected() {
            Some(0) if len > 0 => len - 1,
            Some(i) => i - 1,
            _ if len > 0 => 0,
            _ => return,
        };
        self.state.select(Some(idx));
        self.scroll = self.scroll.position(idx * crate::ITEM_HEIGHT as usize);
    }

    /// Jump to the first row
    pub fn first_row(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
            self.scroll = self.scroll.position(0);
        }
    }

    /// Jump to the last row
    pub fn last_row(&mut self) {
        let len = self.items.len();
        if len > 0 {
            let last = len - 1;
            self.state.select(Some(last));
            self.scroll = self.scroll.position(last * crate::ITEM_HEIGHT as usize);
        }
    }

    /// Page down by visible_rows
    pub fn page_down(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let new = (current + self.visible_rows).min(len - 1);
        self.state.select(Some(new));
        self.scroll = self.scroll.position(new * crate::ITEM_HEIGHT as usize);
    }

    /// Page up by visible_rows
    pub fn page_up(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let new = current.saturating_sub(self.visible_rows);
        self.state.select(Some(new));
        self.scroll = self.scroll.position(new * crate::ITEM_HEIGHT as usize);
    }

    /// Render the table and its scrollbar
    pub fn render(&mut self, frame: &mut Frame, area: Rect, colors: &TableColors) {
        // Compute how many rows fit
        self.visible_rows = area.height.saturating_sub(1) as usize;

        // Build header
        let header =
            Row::new(["Port", "PID", "Process Name", "Process Path", "Listener"].map(Cell::from))
                .style(Style::default().fg(colors.header_fg).bg(colors.header_bg))
                .height(crate::ITEM_HEIGHT);

        // Build rows
        let rows = self.items.iter().map(|item| {
            Row::new(item.ref_array().into_iter().map(Cell::from))
                .style(Style::default())
                .height(crate::ITEM_HEIGHT)
        });

        // Construct table
        let table = Table::new(
            rows,
            [
                Constraint::Length(self.column_widths.0),
                Constraint::Min(self.column_widths.1),
                Constraint::Min(self.column_widths.2),
                Constraint::Min(self.column_widths.3),
                Constraint::Min(self.column_widths.4),
            ],
        )
        .header(header)
        .row_highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(colors.selected_row_style_fg),
        )
        .cell_highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(colors.selected_cell_style_fg),
        )
        .bg(colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);

        // Render table
        frame.render_stateful_widget(table, area, &mut self.state);

        // Render scrollbar
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll,
        );
    }
}
