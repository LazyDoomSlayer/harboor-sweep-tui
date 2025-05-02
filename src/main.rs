mod common;
use crate::common::{KillProcessResponse, PortInfo, ProcessInfo, ProcessInfoResponse};

#[cfg(target_family = "unix")]
pub mod unix;

#[cfg(target_family = "windows")]
pub mod windows;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Position},
    prelude::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Paragraph},
};

use ratatui::layout::Rect;
use std::sync::{Arc, Mutex};

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
    input_mode: InputMode,
    port_process_user_input: String,
    port_process_user_input_character_index: usize,
    is_searching: bool,

    // processes
    processes: Vec<PortInfo>,
    interval: Arc<Mutex<u64>>,
    is_monitoring: Arc<Mutex<bool>>,
}
#[derive(Debug, Default)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

enum AppControlFlow {
    Continue,
    Exit,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self {
            port_process_user_input: String::new(),
            port_process_user_input_character_index: 0,
            input_mode: InputMode::Normal,
            is_searching: false,
            // Processes
            processes: Vec::new(),
            interval: Arc::new(Mutex::new(5)),
            is_monitoring: Arc::new(Mutex::new(false)),
        }
    }

    /// Run the application's main loop.
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;
            self.start_monitoring().expect("TODO: panic message");

            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if matches!(self.handle_key_event(key)?, AppControlFlow::Exit) {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<AppControlFlow> {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode_key(key),
            InputMode::Editing => {
                self.handle_editing_mode_key(key);
                Ok(AppControlFlow::Continue)
            }
        }
    }

    fn handle_normal_mode_key(&mut self, key: KeyEvent) -> Result<AppControlFlow> {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('e')) => {
                self.input_mode = InputMode::Editing;
            }
            (KeyModifiers::NONE, KeyCode::Char('q' | 'Q'))
            | (KeyModifiers::NONE, KeyCode::Esc)
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => {
                return Ok(AppControlFlow::Exit);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f' | 'F')) => {
                self.is_searching = !self.is_searching;
                if self.is_searching {
                    self.input_mode = InputMode::Editing;
                }
            }
            _ => {}
        }
        Ok(AppControlFlow::Continue)
    }

    fn handle_editing_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(to_insert) => self.enter_char(to_insert),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Esc => self.input_mode = InputMode::Normal,
            _ => {}
        }
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        // if self.is_searching {
        //
        // }

        let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]);
        let [input_area, table_area] = vertical.areas(frame.area());

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

        self.draw_process_list(frame, table_area);
    }

    fn draw_process_list(&self, frame: &mut Frame, area: Rect) {
        // Build your ListItem vec
        let processes_listed: Vec<ListItem> = self
            .processes
            .iter()
            .enumerate()
            .map(|(i, proc)| {
                // one Line for the header…
                // let header: Line = Line::from(vec![
                // ]);

                // …and one Line for the details
                let details: Line = Line::from(vec![
                    Span::raw(format!("{}", proc.pid)),
                    Span::raw(format!("{}", proc.port)),
                    Span::raw(format!("{}", proc.process_path)),
                    Span::raw(format!("{}", proc.is_listener)),
                ]);

                ListItem::new(vec![details])
            })
            .collect();

        let processes_widget =
            List::new(processes_listed).block(Block::bordered().title("Processes"));
        frame.render_widget(processes_widget, area);
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

    // Processes
    pub fn set_interval(&self, interval: u64) {
        *self.interval.lock().unwrap() = interval;
    }

    pub fn is_monitoring(&self) -> bool {
        *self.is_monitoring.lock().unwrap()
    }

    pub fn set_monitoring(&self, state: bool) {
        *self.is_monitoring.lock().unwrap() = state;
    }

    fn start_monitoring(&mut self) -> Result<(), String> {
        // if self.is_monitoring {
        //     return Err("Monitoring is already running".into());
        // }

        self.set_monitoring(true);

        self.monitor_ports_loop();

        Ok(())
    }

    fn monitor_ports_loop(&mut self) {
        match self.fetch_ports_by_os() {
            Ok(ports) => {
                // update vtor
                self.processes = ports
            }
            Err(e) => {
                eprintln!("Error fetching ports: {}", e);
            }
        }
    }

    fn fetch_ports_by_os(&self) -> Result<Vec<PortInfo>, String> {
        #[cfg(target_family = "unix")]
        {
            unix::fetch_ports()
        }
        #[cfg(target_family = "windows")]
        {
            windows::fetch_ports()
        }
    }

    fn stop_monitoring(&self) -> Result<(), String> {
        if !self.is_monitoring() {
            return Err("Monitoring is not running".to_string());
        }

        self.set_monitoring(false);
        Ok(())
    }

    fn update_interval(&self, new_interval: u64) -> Result<(), String> {
        if new_interval < 1 || new_interval > 60 {
            return Err("Interval must be between 1 and 60 seconds".to_string());
        }

        self.set_interval(new_interval);

        Ok(())
    }

    fn fetch_ports() -> Result<Vec<PortInfo>, String> {
        #[cfg(target_family = "unix")]
        {
            unix::fetch_ports()
        }
        #[cfg(target_family = "windows")]
        {
            windows::fetch_ports()
        }
    }

    fn kill_process(pid: u32) -> KillProcessResponse {
        #[cfg(target_family = "unix")]
        {
            unix::kill_process(pid)
        }
        #[cfg(target_family = "windows")]
        {
            windows::kill_process(pid)
        }
    }

    fn get_processes_using_port(port: u16, item_pid: u32) -> Result<ProcessInfoResponse, String> {
        #[cfg(target_family = "unix")]
        {
            unix::get_processes_using_port(port, item_pid)
        }
        #[cfg(target_family = "windows")]
        {
            return Ok(ProcessInfoResponse {
                is_listener: false,
                data: Some(ProcessInfo {
                    pid: 5678,
                    port,
                    process_name: "mocked_process.exe".to_string(),
                    process_path: item_pid.to_string(),
                }),
            });
        }
    }
}
