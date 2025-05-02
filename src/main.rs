mod common;
use crate::common::{KillProcessResponse, PortInfo, ProcessInfo, ProcessInfoResponse};
#[cfg(target_family = "unix")]
pub mod unix;

#[cfg(target_family = "windows")]
pub mod windows;
use color_eyre::Result;

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout, Margin, Position, Rect},
    prelude::{Color, Style},
    style::{self, Modifier, Stylize, palette::tailwind},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, List, ListItem, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
};

use std::sync::{Arc, Mutex};
use unicode_width::UnicodeWidthStr;

const PALETTES: [tailwind::Palette; 5] = [
    tailwind::GRAY,
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: [&str; 1] =
    ["(Esc) quit | (↑) move up | (↓) move down | (←) move left | (→) move right"];
// "(Shift + →) next color | (Shift + ←) previous color",

const ITEM_HEIGHT: u16 = 2;

#[derive(Debug, Default)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

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

    // Table list
    state: TableState,
    scroll_state: ScrollbarState,
    longest_item_lens: (u16, u16, u16, u16, u16), // order is (port pid process_name process_path is_listener)
    colors: TableColors,
    color_index: usize,
}

impl PortInfo {
    pub fn ref_array(&self) -> Vec<String> {
        vec![
            self.port.to_string(),
            self.pid.to_string(),
            self.process_name.clone(),
            self.process_path.clone(),
            self.is_listener.to_string(),
        ]
    }

    fn pid(&self) -> &u32 {
        &self.pid
    }
    fn process_name(&self) -> &str {
        &self.process_name
    }
    fn process_path(&self) -> &str {
        &self.process_path
    }
    fn port(&self) -> &u16 {
        &self.port
    }
    fn is_listener(&self) -> &bool {
        &self.is_listener
    }
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
            // Table list
            state: TableState::default(),
            scroll_state: ScrollbarState::new((1 * ITEM_HEIGHT) as usize),
            longest_item_lens: (5, 5, 30, 55, 5),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
        }
    }

    /// Table list
    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.processes.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT as usize);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.processes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT as usize);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }
    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    /// Run the application's main loop.
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.start_monitoring().expect("TODO: panic message");

        loop {
            terminal.draw(|frame| self.render(frame))?;

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
        let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);

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
            (_, KeyCode::Char('j') | KeyCode::Down) => self.next_row(),
            (_, KeyCode::Char('k') | KeyCode::Up) => self.previous_row(),
            (_, KeyCode::Char('l') | KeyCode::Right) if shift_pressed => self.next_color(),
            (_, KeyCode::Char('h') | KeyCode::Left) if shift_pressed => {
                self.previous_color();
            }
            // (_, KeyCode::Char('l') | KeyCode::Right) => self.next_column(),
            // (_, KeyCode::Char('h') | KeyCode::Left) => self.previous_column(),
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
        let vertical = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ]);
        let [input_area, table_area, table_footer] = vertical.areas(frame.area());

        let input = Paragraph::new(self.port_process_user_input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.colors.footer_border_color))
                    .title(" Search "),
            );

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

        self.set_colors();

        self.render_table(frame, table_area);
        self.render_scrollbar(frame, table_area);
        self.render_footer(frame, table_footer);
    }
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = ["PID", "Port", "Process Name", "Process Path", "Listener"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.processes.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new()) // .fg(self.colors.row_fg).bg(color)
                .height(ITEM_HEIGHT)
        });

        let t = Table::new(
            rows,
            [
                Constraint::Length(self.longest_item_lens.0),
                Constraint::Min(self.longest_item_lens.1),
                Constraint::Min(self.longest_item_lens.2),
                Constraint::Min(self.longest_item_lens.3),
                Constraint::Min(self.longest_item_lens.4),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .column_highlight_style(selected_col_style)
        .cell_highlight_style(selected_cell_style)
        .bg(self.colors.buffer_bg)
        .block(
            Block::bordered()
                .border_type(BorderType::Plain)
                .border_style(Style::new().fg(self.colors.footer_border_color)),
        )
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(t, area, &mut self.state);
    }

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
            &mut self.scroll_state,
        );
    }
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );
        frame.render_widget(info_footer, area);
    }
    //
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
                self.processes = ports;
                println!("{:?}", self.processes.len());
                self.scroll_state =
                    ScrollbarState::new((self.processes.len() * ITEM_HEIGHT as usize));
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

fn constraint_len_calculator(processes: &[PortInfo]) -> (u16, u16, u16, u16, u16) {
    let pid = processes
        .iter()
        .map(|p| p.pid.to_string().width())
        .max()
        .unwrap_or(0);

    let port = processes
        .iter()
        .map(|p| p.port.to_string().width())
        .max()
        .unwrap_or(0);

    let process_name = processes
        .iter()
        .map(|p| p.process_name.width())
        .max()
        .unwrap_or(0);

    let process_path = processes
        .iter()
        .map(|p| p.process_path.width())
        .max()
        .unwrap_or(0);

    let is_listener = processes
        .iter()
        .map(|p| p.is_listener.to_string().width())
        .max()
        .unwrap_or(0);

    // truncate into u16 (should be safe for typical terminal widths)
    (
        pid as u16,
        port as u16,
        process_name as u16,
        process_path as u16,
        is_listener as u16,
    )
}
