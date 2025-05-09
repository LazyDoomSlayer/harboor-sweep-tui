use crate::ui::theme::TableColors;
use crate::util::{keybindings_constraint_len_calculator, popup_area};
use ratatui::{
    Frame,
    layout::{Constraint, Margin, Rect},
    prelude::Style,
    style::{Modifier, Stylize},
    widgets::{
        Block, BorderType, Cell, Clear, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};

#[derive(Debug)]
pub struct Keybinding {
    combo: String,
    description: String,
}
impl Keybinding {
    pub fn ref_array(&self) -> Vec<String> {
        vec![self.combo.to_string(), self.description.to_string()]
    }

    pub fn combo(&self) -> &str {
        &self.combo
    }
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// A component that handles the help/keybindings popup
#[derive(Debug)]
pub struct KeybindingsComponent {
    /// List of keybindings (combo, description)
    items: Vec<Keybinding>,
    /// Whether the popup is displayed
    pub display: bool,
    /// Table selection state
    pub state: TableState,
    /// Scrollbar state
    pub scroll: ScrollbarState,
    /// Number of visible rows
    pub visible_rows: usize,
    /// Column width constraints (combo, description)
    pub col_widths: (u16, u16),
}

impl Default for KeybindingsComponent {
    fn default() -> Self {
        let items = vec![
            Keybinding {
                combo: "Esc / q / Ctrl+C".into(),
                description: "Quit the application".into(),
            },
            Keybinding {
                combo: "Ctrl+F".into(),
                description: "Toggle the search input".into(),
            },
            Keybinding {
                combo: "F1 / ?".into(),
                description: "Show or hide this help dialog".into(),
            },
            Keybinding {
                combo: "j / ↓".into(),
                description: "Move selection down".into(),
            },
            Keybinding {
                combo: "k / ↑".into(),
                description: "Move selection up".into(),
            },
            Keybinding {
                combo: "PageDown".into(),
                description: "Page down".into(),
            },
            Keybinding {
                combo: "PageUp".into(),
                description: "Page up".into(),
            },
            Keybinding {
                combo: "Shift+PageDown".into(),
                description: "Jump to last item".into(),
            },
            Keybinding {
                combo: "Shift+PageUp".into(),
                description: "Jump to first item".into(),
            },
            Keybinding {
                combo: "Shift+Right / l".into(),
                description: "Next color theme".into(),
            },
            Keybinding {
                combo: "Shift+Left / h".into(),
                description: "Previous color theme".into(),
            },
            Keybinding {
                combo: "e".into(),
                description: "Enter editing mode".into(),
            },
        ];
        let col_widths = keybindings_constraint_len_calculator(&items);
        Self {
            items,
            display: false,
            state: TableState::default(),
            scroll: ScrollbarState::new(1),
            visible_rows: 0,
            col_widths,
        }
    }
}

impl KeybindingsComponent {
    /// Toggle display on/off, clear selection when opening
    pub fn toggle(&mut self) {
        self.display = !self.display;
        if self.display {
            self.state.select(Some(0));
            self.scroll = self.scroll.position(0);
        }
    }

    /// Move selection down by one row
    pub fn next_row(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let idx = match self.state.selected() {
            Some(i) if i + 1 < len => i + 1,
            _ => 0,
        };
        self.state.select(Some(idx));
        self.scroll = self.scroll.position(idx * crate::ITEM_HEIGHT as usize);
    }

    /// Move selection up by one row
    pub fn previous_row(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let idx = match self.state.selected() {
            Some(0) => len - 1,
            Some(i) => i - 1,
            _ => 0,
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

    /// Page down
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

    /// Page up
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

    /// Render the keybindings popup
    pub fn render(&mut self, frame: &mut Frame, area: Rect, colors: &TableColors) {
        // Update visible rows
        self.visible_rows = area.height.saturating_sub(1) as usize;

        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(colors.selected_row_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(colors.selected_cell_style_fg);

        let combo_style = Style::new()
            .fg(colors.selected_row_style_fg)
            .bg(colors.buffer_bg);
        let desc_style = Style::new().fg(colors.row_fg).bg(colors.buffer_bg);

        let rows = self.items.iter().map(|kb| {
            let cells = kb.ref_array().into_iter().enumerate().map(|(i, c)| {
                let cell = Cell::from(c);
                if i == 0 {
                    cell.style(combo_style)
                } else {
                    cell.style(desc_style)
                }
            });
            Row::new(cells).height(crate::ITEM_HEIGHT)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Length(self.col_widths.0 + 1),
                Constraint::Min(self.col_widths.1),
            ],
        )
        .row_highlight_style(selected_row_style)
        .cell_highlight_style(selected_cell_style)
        .bg(colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always)
        .block(
            Block::bordered()
                .border_type(BorderType::Plain)
                .border_style(Style::new().fg(colors.footer_border_color))
                .title("Keybindings"),
        );
        let area = popup_area(area, 4, 5);

        frame.render_widget(Clear, area);
        frame.render_stateful_widget(table, area, &mut self.state);

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
