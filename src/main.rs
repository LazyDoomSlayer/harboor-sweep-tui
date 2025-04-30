use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::prelude::{Color, Style};
use ratatui::widgets::Block;
use ratatui::{widgets::Paragraph, DefaultTerminal, Frame};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    input_mode: InputMode,
    port_process_user_input: String,
    port_process_user_input_character_index: usize,
    is_searching: bool,
}
#[derive(Debug, Default)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}
impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self {
            running: false,
            port_process_user_input: String::new(),
            port_process_user_input_character_index: 0,
            input_mode: InputMode::Normal,
            is_searching: false,
        }
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // self.running = true;
        // while self.running {
        //     terminal.draw(|frame| self.render(frame))?;

        // }
        // Ok(())
        loop {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;

            if let Event::Key(key) = event::read()? {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        if self.is_searching {
            let vertical = Layout::vertical([Constraint::Length(3)]);
            let [input_area] = vertical.areas(frame.area());

            let input = Paragraph::new(self.port_process_user_input.as_str())
                .style(match self.input_mode {
                    InputMode::Normal => Style::default(),
                    InputMode::Editing => Style::default().fg(Color::Yellow),
                })
                .block(Block::bordered().title(" Search "));

            frame.render_widget(input, input_area);

            match self.input_mode {
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                InputMode::Normal => {}

                // Make the cursor visible and ask ratatui to put it at the specified coordinates after
                // rendering
                #[allow(clippy::cast_possible_truncation)]
                InputMode::Editing => frame.set_cursor_position(Position::new(
                    // Draw the cursor at the current position in the input field.
                    // This position is can be controlled via the left and right arrow key
                    input_area.x + self.port_process_user_input_character_index as u16 + 1,
                    // Move one line down, from the border to the input line
                    input_area.y + 1,
                )),
            }
        }

        // let text = "Hello, Harboor Sweep TUI!";
        // frame.render_widget(Paragraph::new(text).centered(), frame.area())
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),

            (_, KeyCode::Char('f')) => {
                self.is_searching = !self.is_searching;
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.port_process_user_input.chars().count())
    }
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self
            .port_process_user_input_character_index
            .saturating_sub(1);
        self.port_process_user_input_character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self
            .port_process_user_input_character_index
            .saturating_add(1);
        self.port_process_user_input_character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn byte_index(&self) -> usize {
        self.port_process_user_input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.port_process_user_input_character_index)
            .unwrap_or(self.port_process_user_input.len())
    }
    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.port_process_user_input.insert(index, new_char);
        self.move_cursor_right();
    }
    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.port_process_user_input_character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.port_process_user_input_character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self
                .port_process_user_input
                .chars()
                .take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.port_process_user_input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.port_process_user_input =
                before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }
}
