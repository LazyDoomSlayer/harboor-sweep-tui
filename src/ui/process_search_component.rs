use crate::ApplicationMode;
use crate::ui::theme::TableColors;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::Style,
    widgets::{Block, BorderType, Paragraph},
};

/// A component that handles the search input state and rendering.
#[derive(Debug)]
pub struct ProcessSearchComponent {
    /// Current input value
    pub value: String,
    /// Cursor position in terms of character index
    pub cursor_index: usize,
    /// Whether the search input is displayed
    pub display: bool,
}

impl Default for ProcessSearchComponent {
    fn default() -> Self {
        Self {
            value: String::new(),
            cursor_index: 0,
            display: false,
        }
    }
}

impl ProcessSearchComponent {
    /// Clears the input and resets cursor
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_index = 0;
    }

    pub fn toggle(&mut self) {
        self.display = !self.display;
        self.clear();
    }

    /// Clamps a proposed cursor position to valid range
    fn clamp_cursor(&self, pos: usize) -> usize {
        pos.clamp(0, self.value.chars().count())
    }

    /// Moves cursor one position left
    pub fn move_cursor_left(&mut self) {
        let new_idx = self.cursor_index.saturating_sub(1);
        self.cursor_index = self.clamp_cursor(new_idx);
    }

    /// Moves cursor one position right
    pub fn move_cursor_right(&mut self) {
        let new_idx = self.cursor_index.saturating_add(1);
        self.cursor_index = self.clamp_cursor(new_idx);
    }

    /// Returns the byte index corresponding to the char cursor
    fn byte_index(&self) -> usize {
        self.value
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.cursor_index)
            .unwrap_or(self.value.len())
    }

    /// Inserts a character at the current cursor position
    pub fn insert_char(&mut self, c: char) {
        let idx = self.byte_index();
        self.value.insert(idx, c);
        self.move_cursor_right();
    }

    /// Deletes the character before the cursor
    pub fn delete_char(&mut self) {
        if self.cursor_index > 0 {
            let before = self.value.chars().take(self.cursor_index - 1);
            let after = self.value.chars().skip(self.cursor_index);
            self.value = before.chain(after).collect();
            self.move_cursor_left();
        }
    }

    /// Renders the search input box
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        colors: &TableColors,
        mode: &ApplicationMode,
    ) {
        let input = Paragraph::new(self.value.as_str())
            .style(Style::default().fg(colors.row_fg).bg(colors.buffer_bg))
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(colors.footer_border_color))
                    .title("Search"),
            );

        frame.render_widget(input, area);

        if matches!(mode, ApplicationMode::Editing) {
            // Place cursor inside input
            frame.set_cursor_position(Position::new(
                area.x + self.cursor_index as u16 + 1,
                area.y + 1,
            ));
        }
    }
}
