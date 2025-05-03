mod common;
use crate::common::{
    KillProcessResponse, PortInfo, ProcessInfo, ProcessInfoResponse, ProcessPortState,
};
#[cfg(target_family = "unix")]
pub mod unix;

#[cfg(target_family = "windows")]
pub mod windows;

use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Flex, Layout, Margin, Position, Rect},
    prelude::{Color, Style},
    style::{self, Modifier, Stylize, palette::tailwind},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Cell, Clear, HighlightSpacing, List, ListItem, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
};

use std::{sync::mpsc, thread, time};

const PALETTES: [tailwind::Palette; 5] = [
    tailwind::GRAY,
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: [&str; 1] =
    ["(Esc) quit | (↑) move up | (↓) move down | (←) move left | (→) move right"];

const ITEM_HEIGHT: u16 = 1;

#[derive(Debug, Default)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
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
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let (event_tx, event_rx) = mpsc::channel::<MultithreadingEvent>();
    let tx_to_input_events = event_tx.clone();
    thread::spawn(move || {
        handle_input_events(tx_to_input_events);
    });
    let tx_to_background_thread = event_tx.clone();
    thread::spawn(move || {
        run_background_thread(tx_to_background_thread);
    });

    let terminal = ratatui::init();
    let result = App::new().run(terminal, event_rx);
    ratatui::restore();
    result
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    // Search widget
    input_mode: InputMode,
    port_process_user_input: String,
    port_process_user_input_character_index: usize,
    is_searching: bool,
    // Help Widget
    show_help: bool,

    // processes
    processes: Vec<PortInfo>,
    filtered_processes: Vec<PortInfo>,
    is_monitoring: bool,

    // Proccess Table list
    state: TableState,
    scroll_state: ScrollbarState,
    longest_item_lens: (u16, u16, u16, u16, u16),
    colors: TableColors,
    color_index: usize,
    visible_rows: usize,
}

enum MultithreadingEvent {
    Crossterm(Event),
    ProccesesUpdate(Vec<PortInfo>),
}

fn handle_input_events(tx: mpsc::Sender<MultithreadingEvent>) {
    loop {
        let evt = match event::read() {
            Ok(evt) => evt,
            Err(e) => {
                eprintln!("Error reading crossterm event: {}", e);
                break;
            }
        };

        let msg = MultithreadingEvent::Crossterm(evt);
        if tx.send(msg).is_err() {
            break;
        }
    }
}

fn run_background_thread(tx: mpsc::Sender<MultithreadingEvent>) {
    loop {
        let event = MultithreadingEvent::ProccesesUpdate(Vec::new());
        tx.send(event).unwrap();

        thread::sleep(time::Duration::from_millis(2_000));
    }
}

impl PortInfo {
    pub fn ref_array(&self) -> Vec<String> {
        vec![
            self.port.to_string(),
            self.pid.to_string(),
            self.process_name.clone(),
            self.process_path.clone(),
            format!("{:?}", self.port_state),
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
    fn port_state(&self) -> &ProcessPortState {
        &self.port_state
    }
}

#[derive(Debug, Default)]
enum InputMode {
    #[default]
    Normal,
    Editing,
    Helping,
}

enum AppControlFlow {
    Continue,
    Exit,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self {
            // Search widget
            port_process_user_input: String::new(),
            port_process_user_input_character_index: 0,
            input_mode: InputMode::Normal,
            is_searching: false,
            // Help Widget
            show_help: false,
            // Processes
            processes: Vec::new(),
            filtered_processes: Vec::new(),
            is_monitoring: false,
            // Table list
            state: TableState::default(),
            scroll_state: ScrollbarState::new((1 * ITEM_HEIGHT) as usize),
            longest_item_lens: (5, 5, 30, 55, 5),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            visible_rows: 0,
        }
    }

    /// Table list
    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.filtered_processes.len() - 1 {
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
                    self.filtered_processes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT as usize);
    }

    pub fn go_to_first(&mut self) {
        if !self.filtered_processes.is_empty() {
            self.state.select(Some(0));
            self.scroll_state = self.scroll_state.position(0);
        }
    }

    pub fn go_to_last(&mut self) {
        let len = self.filtered_processes.len();
        if len > 0 {
            let last = len - 1;
            self.state.select(Some(last));
            self.scroll_state = self.scroll_state.position(last * ITEM_HEIGHT as usize);
        }
    }

    pub fn page_down(&mut self) {
        let len = self.filtered_processes.len();
        if len == 0 {
            return;
        }

        let current = self.state.selected().unwrap_or(0);
        // move down by one screenful, clamped to last row
        let new = (current + self.visible_rows).min(len - 1);

        self.state.select(Some(new));
        self.scroll_state = self.scroll_state.position(new * ITEM_HEIGHT as usize);
    }

    pub fn page_up(&mut self) {
        let len = self.filtered_processes.len();
        if len == 0 {
            return;
        }

        let current = self.state.selected().unwrap_or(0);
        // move up by one screenful, clamped at zero
        let new = current.saturating_sub(self.visible_rows);

        self.state.select(Some(new));
        self.scroll_state = self.scroll_state.position(new * ITEM_HEIGHT as usize);
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
    fn run(
        mut self,
        mut terminal: DefaultTerminal,
        rx: mpsc::Receiver<MultithreadingEvent>,
    ) -> Result<()> {
        loop {
            match rx.recv().unwrap() {
                MultithreadingEvent::Crossterm(event) => match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if matches!(self.handle_key_event(key)?, AppControlFlow::Exit) {
                            return Ok(());
                        }
                    }
                    _ => {}
                },
                MultithreadingEvent::ProccesesUpdate(_data) => self.monitor_ports_loop(),
            }

            terminal.draw(|frame| self.render(frame))?;
        }
    }

    fn update_filtered_processes(&mut self) {
        let q = self.port_process_user_input.to_lowercase();
        self.filtered_processes = self
            .processes
            .iter()
            .filter(|p| {
                // match pid
                p.pid.to_string().contains(&q)
                    || p.port.to_string().contains(&q)
                    || p.process_name.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<AppControlFlow> {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode_key(key),
            InputMode::Editing => {
                self.handle_editing_mode_key(key);
                Ok(AppControlFlow::Continue)
            }
            InputMode::Helping => {
                self.handle_helping_mode_key(key);
                Ok(AppControlFlow::Continue)
            }
        }
    }

    fn handle_normal_mode_key(&mut self, key: KeyEvent) -> Result<AppControlFlow> {
        match (key.modifiers, key.code) {
            // Quit from application
            (KeyModifiers::NONE, KeyCode::Char('q' | 'Q'))
            | (KeyModifiers::NONE, KeyCode::Esc)
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => {
                return Ok(AppControlFlow::Exit);
            }
            // Toggle UI elements
            (KeyModifiers::CONTROL, KeyCode::Char('f' | 'F')) => {
                self.is_searching = !self.is_searching;
                self.clear_input();

                if self.is_searching {
                    self.input_mode = InputMode::Editing;
                }
            }
            (KeyModifiers::NONE, KeyCode::F(1)) => {
                self.show_help = !self.show_help;

                if (self.show_help) {
                    self.input_mode = InputMode::Helping;
                } else {
                    self.input_mode = InputMode::Normal;
                }
            }
            // Modify Search input mode
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                self.input_mode = InputMode::Editing;
            }
            // Navigate in the list
            (KeyModifiers::SHIFT, KeyCode::PageUp) => self.go_to_first(),
            (KeyModifiers::SHIFT, KeyCode::PageDown) => self.go_to_last(),
            (KeyModifiers::NONE, KeyCode::PageUp) => self.page_up(),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.page_down(),
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => self.next_row(),
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => self.previous_row(),
            // Change theme
            (KeyModifiers::SHIFT, KeyCode::Char('l') | KeyCode::Right) => self.next_color(),
            (KeyModifiers::SHIFT, KeyCode::Char('h') | KeyCode::Left) => {
                self.previous_color();
            }
            _ => {}
        }
        Ok(AppControlFlow::Continue)
    }

    fn handle_helping_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.show_help = !self.show_help;
                if (self.show_help) {
                    self.input_mode = InputMode::Helping;
                } else {
                    self.input_mode = InputMode::Normal;
                }
            }
            _ => {}
        }
    }
    fn handle_editing_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(to_insert) => self.enter_char(to_insert),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Down => {
                self.input_mode = InputMode::Normal;
                self.next_row()
            }
            KeyCode::Up => {
                self.input_mode = InputMode::Normal;
                self.previous_row()
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.is_searching = !self.is_searching;

                self.clear_input();

                if self.is_searching {
                    self.input_mode = InputMode::Editing;
                }
            }
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
        self.set_colors();
        let area = frame.area();

        if !self.is_searching {
            let [table_area] = Layout::vertical([Constraint::Min(1)]).areas(area);
            self.visible_rows = table_area.height as usize - 1;
            self.render_table(frame, table_area);
            self.render_scrollbar(frame, table_area);
        } else {
            let [input_area, table_area] =
                Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(area);

            self.visible_rows = table_area.height as usize - 1;

            self.render_search(frame, input_area);
            self.render_table(frame, table_area);
            self.render_scrollbar(frame, table_area);
        }

        self.render_help_popup(frame, area);
    }
    /// helper function to create a centered rect using up certain percentage of the available rect `r`
    fn popup_area(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    fn render_help_popup(&mut self, frame: &mut Frame, area: Rect) {
        if self.show_help {
            let block = Block::bordered()
                .border_type(BorderType::Plain)
                .border_style(Style::new().fg(self.colors.footer_border_color))
                .title("Keybindings");
            let area = self.popup_area(area, 60, 20);
            frame.render_widget(Clear, area);
            frame.render_widget(block, area);
        }
    }

    fn render_search(&mut self, frame: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.port_process_user_input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
                InputMode::Editing => Style::default()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
                InputMode::Helping => Style::default()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            })
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.colors.footer_border_color))
                    .title(" Search "),
            );

        frame.render_widget(input, area);

        match self.input_mode {
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            InputMode::Normal => {}
            InputMode::Helping => {}

            // Make the cursor visible and ask ratatui to put it at the specified coordinates after
            // rendering
            #[allow(clippy::cast_possible_truncation)]
            InputMode::Editing => frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                area.x + self.port_process_user_input_character_index as u16 + 1,
                // Move one line down, from the border to the input line
                area.y + 1,
            )),
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = ["PID", "Port", "Process Name", "Process Path", "Listener"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.filtered_processes.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("{content}"))))
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
        .cell_highlight_style(selected_cell_style)
        .bg(self.colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);

        // .block(
        //     Block::bordered()
        //         .border_type(BorderType::Plain)
        //         .border_style(Style::new().fg(self.colors.footer_border_color)),
        // )

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
                    Span::raw(format!("{:?}", proc.port_state)),
                ]);

                ListItem::new(vec![details])
            })
            .collect();

        let processes_widget =
            List::new(processes_listed).block(Block::bordered().title("Processes"));
        frame.render_widget(processes_widget, area);
    }

    fn clear_input(&mut self) {
        self.port_process_user_input.clear();
        self.port_process_user_input_character_index = 0;
        self.update_filtered_processes();
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
        self.update_filtered_processes();
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
            self.update_filtered_processes();
        }
    }

    fn monitor_ports_loop(&mut self) {
        match self.fetch_ports_by_os() {
            Ok(ports) => {
                self.processes = ports;
                self.update_filtered_processes();
                let length = self.filtered_processes.len() * ITEM_HEIGHT as usize;
                self.scroll_state = self.scroll_state.content_length(length);
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
                port_state: ProcessPortState::Using,
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
