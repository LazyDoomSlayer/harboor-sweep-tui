use crate::ApplicationMode;
use crate::ui::theme::TableColors;
use crate::util::{center_str, popup_area};
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

/// Represents a single key combo and its description.
#[derive(Debug)]
pub struct Keybinding {
    pub combo: &'static str,
    pub description: &'static str,
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

/// Groups keybindings by application mode.
#[derive(Debug)]
pub struct KeybindingsGroup {
    pub mode: ApplicationMode,
    pub bindings: Vec<Keybinding>,
}

/// Returns the full set of keybindings, divided by mode.
pub fn default_keybindings() -> Vec<KeybindingsGroup> {
    vec![
        KeybindingsGroup {
            mode: ApplicationMode::Helping,
            bindings: vec![
                Keybinding {
                    combo: "Esc, F1, ?",
                    description: "Exit help view",
                },
                Keybinding {
                    combo: "Up, Down",
                    description: "Navigate help entries",
                },
                Keybinding {
                    combo: "Pg Up, Pg Down",
                    description: "Page through help list",
                },
                Keybinding {
                    combo: "Shift+Pg Up, Shift+Pg Down",
                    description: "Jump to start/end of help list",
                },
            ],
        },
        KeybindingsGroup {
            mode: ApplicationMode::Normal,
            bindings: vec![
                Keybinding {
                    combo: "Esc, q, Ctrl+C",
                    description: "Quit the application",
                },
                Keybinding {
                    combo: "Ctrl+F",
                    description: "Toggle search input display",
                },
                Keybinding {
                    combo: "F1, ?",
                    description: "Toggle keybindings help",
                },
                Keybinding {
                    combo: "e",
                    description: "Enter editing mode (search)",
                },
                Keybinding {
                    combo: "Up, Down",
                    description: "Move selection in table",
                },
                Keybinding {
                    combo: "Pg Up, Pg Down",
                    description: "Scroll one page in table",
                },
                Keybinding {
                    combo: "1",
                    description: "Sort by Port, press again to toggle direction",
                },
                Keybinding {
                    combo: "2",
                    description: "Sort by PID, press again to toggle direction",
                },
                Keybinding {
                    combo: "3",
                    description: "Sort by Process Name, press again to toggle direction",
                },
                Keybinding {
                    combo: "4",
                    description: "Sort by Process Path, press again to toggle direction",
                },
                Keybinding {
                    combo: "Shift+Pg Up, Shift+Pg Down",
                    description: "Jump to start/end of table",
                },
                Keybinding {
                    combo: "k",
                    description: "Open kill-process confirmation for selected row",
                },
                Keybinding {
                    combo: "Shift+Right, Shift+Left",
                    description: "Cycle through available themes",
                },
            ],
        },
        KeybindingsGroup {
            mode: ApplicationMode::Editing,
            bindings: vec![
                Keybinding {
                    combo: "Char keys (a–z, 0–9)",
                    description: "Insert character into search field",
                },
                Keybinding {
                    combo: "Backspace",
                    description: "Delete character from search field",
                },
                Keybinding {
                    combo: "Left, Right",
                    description: "Move cursor in search input",
                },
                Keybinding {
                    combo: "Down",
                    description: "Submit search and move selection down",
                },
                Keybinding {
                    combo: "Up",
                    description: "Submit search and move selection up",
                },
                Keybinding {
                    combo: "Esc",
                    description: "Exit search editing (hide input)",
                },
            ],
        },
        KeybindingsGroup {
            mode: ApplicationMode::Killing,
            bindings: vec![
                Keybinding {
                    combo: "Left",
                    description: "Select 'Kill' action",
                },
                Keybinding {
                    combo: "Right",
                    description: "Select 'Cancel' action",
                },
                Keybinding {
                    combo: "Enter",
                    description: "Confirm selected kill/cancel action",
                },
                Keybinding {
                    combo: "Esc",
                    description: "Abort kill & close confirmation",
                },
            ],
        },
    ]
}
/// Internal helper: either a section‐header or an actual keybinding entry
#[derive(Debug)]
enum KeybindingRow {
    Section(&'static str),
    Entry {
        combo: &'static str,
        description: &'static str,
    },
}

impl KeybindingRow {
    fn cells(&self) -> [&str; 2] {
        match self {
            KeybindingRow::Section(title) => [*title, ""],
            KeybindingRow::Entry { combo, description } => [*combo, *description],
        }
    }
    fn is_section(&self) -> bool {
        matches!(self, KeybindingRow::Section(_))
    }
}

/// A component that handles the help/keybindings popup
#[derive(Debug)]
pub struct KeybindingsComponent {
    /// Flattened list of sections + entries
    items: Vec<KeybindingRow>,
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
        let mut items = Vec::new();
        for KeybindingsGroup { mode, bindings } in default_keybindings() {
            let header = match mode {
                ApplicationMode::Helping => "---- LOCAL ----",
                ApplicationMode::Normal => "---- NORMAL ----",
                ApplicationMode::Editing => "---- SEARCHING ----",
                ApplicationMode::Killing => "---- KILLING ----",
            };
            items.push(KeybindingRow::Section(header));
            for kb in bindings {
                items.push(KeybindingRow::Entry {
                    combo: kb.combo,
                    description: kb.description,
                });
            }
        }

        Self {
            items,
            display: false,
            state: TableState::default(),
            scroll: ScrollbarState::new(1),
            visible_rows: 0,
            col_widths: (30, 70),
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

        let header = Row::new(
            [
                center_str("Key:", self.col_widths.0),
                "Description:".to_string(),
            ]
            .map(Cell::from),
        )
        .height(crate::ITEM_HEIGHT);

        let rows = self.items.iter().enumerate().map(|(i, row)| {
            let [left, right] = row.cells();

            let is_selected = Some(i) == self.state.selected();
            let style = match (row.is_section(), is_selected) {
                (true, _) => Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(colors.selected_row_style_fg),
                (false, true) => Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(colors.selected_row_style_fg),
                (false, false) => Style::default().fg(colors.row_fg),
            };

            let cells = vec![
                Cell::from(center_str(left, self.col_widths.0)).style(style),
                Cell::from(right).style(style),
            ];
            Row::new(cells).height(crate::ITEM_HEIGHT)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Min(self.col_widths.0 + 1),
                Constraint::Min(self.col_widths.1),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .cell_highlight_style(selected_cell_style)
        .bg(colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always)
        .block(
            Block::bordered()
                .border_type(BorderType::Plain)
                .border_style(Style::new().fg(colors.footer_border_color))
                .title(" Keybindings "),
        );
        let area = popup_area(area, 7, 5);

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
