use crate::model::PortInfo;
use crate::ui::theme::TableColors;

use ratatui::widgets::ScrollbarOrientation;
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    style::{Modifier, Style, Stylize},
    widgets::HighlightSpacing,
    widgets::{Cell, Row, Scrollbar, ScrollbarState, Table, TableState},
};

#[derive(Debug, Copy, PartialEq, Default, Clone)]
pub enum SortBy {
    #[default]
    Port,
    PID,
    ProcessName,
    ProcessPath,
}

#[derive(Debug, Copy, PartialEq, Default, Clone)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

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
    /// Sorting state by column
    pub sort_by: SortBy,
    /// Sorting direction
    pub sort_direction: SortDirection,
}

impl Default for ProcessTableComponent {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            state: TableState::default(),
            scroll: ScrollbarState::new(1),
            visible_rows: 0,
            column_widths: (6, 6, 23, 50, 10), // Port, PID, ProcessName, ProcessPath, Listener
            sort_by: SortBy::Port,
            sort_direction: SortDirection::Ascending,
        }
    }
}

impl ProcessTableComponent {
    /// Replace current items and update scrollbar length
    pub fn set_items(&mut self, items: Vec<PortInfo>) {
        self.items = items;
        self.sort_items();
        let content_len = self.items.len() * crate::ITEM_HEIGHT as usize;
        self.scroll = self.scroll.content_length(content_len);
    }
    /// Sort items by current sort criteria
    pub fn sort_items(&mut self) {
        match (self.sort_by, self.sort_direction) {
            (SortBy::Port, SortDirection::Ascending) => self.items.sort_by_key(|i| i.port),
            (SortBy::Port, SortDirection::Descending) => {
                self.items.sort_by_key(|i| std::cmp::Reverse(i.port))
            }
            (SortBy::PID, SortDirection::Ascending) => self.items.sort_by_key(|i| i.pid),
            (SortBy::PID, SortDirection::Descending) => {
                self.items.sort_by_key(|i| std::cmp::Reverse(i.pid))
            }
            (SortBy::ProcessName, SortDirection::Ascending) => self.items.sort_by(|a, b| {
                a.process_name
                    .to_lowercase()
                    .cmp(&b.process_name.to_lowercase())
            }),
            (SortBy::ProcessName, SortDirection::Descending) => self.items.sort_by(|a, b| {
                b.process_name
                    .to_lowercase()
                    .cmp(&a.process_name.to_lowercase())
            }),
            (SortBy::ProcessPath, SortDirection::Ascending) => self.items.sort_by(|a, b| {
                a.process_path
                    .to_lowercase()
                    .cmp(&b.process_path.to_lowercase())
            }),
            (SortBy::ProcessPath, SortDirection::Descending) => self.items.sort_by(|a, b| {
                b.process_path
                    .to_lowercase()
                    .cmp(&a.process_path.to_lowercase())
            }),
        }
    }
    /// Set sort column and toggle sort direction if it's already set to this column
    pub fn set_or_toggle_sort(&mut self, by: SortBy) {
        if self.sort_by == by {
            self.toggle_sort_direction(None);
        } else {
            self.set_sort_column(by);
        }
    }

    /// Set sort column and reset sort direction to ascending
    pub fn set_sort_column(&mut self, by: SortBy) {
        if self.sort_by != by {
            self.sort_by = by;
            self.sort_direction = SortDirection::Ascending;
            self.sort_items();
        }
    }

    /// Toggle or explicitly set sort direction
    pub fn toggle_sort_direction(&mut self, direction: Option<SortDirection>) {
        self.sort_direction = match direction {
            Some(dir) => dir,
            None => match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            },
        };
        self.sort_items();
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

    /// Display direction indicator if sorting by this column
    fn header_with_sort(&self, title: &str, column: SortBy) -> String {
        if self.sort_by == column {
            let arrow = match self.sort_direction {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            };

            format!("{}{}", title, arrow)
        } else {
            title.to_string()
        }
    }

    /// Render the table and its scrollbar
    pub fn render(&mut self, frame: &mut Frame, area: Rect, colors: &TableColors) {
        // Compute how many rows fit
        self.visible_rows = area.height.saturating_sub(1) as usize;

        // Build header
        let headers = [
            self.header_with_sort("Port", SortBy::Port),
            self.header_with_sort("PID", SortBy::PID),
            self.header_with_sort("Process Name", SortBy::ProcessName),
            self.header_with_sort("Process Path", SortBy::ProcessPath),
            "Listener".to_string(), // No need to sort this one
        ];

        let header = Row::new(headers.map(Cell::from))
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
                Constraint::Length(self.column_widths.1),
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
        self.render_scrollbar(frame, area);
    }
    /// Renders scrollbar for table list
    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
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
