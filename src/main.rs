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

use ratatui::layout::Direction;
use ratatui::widgets::Wrap;
use std::{sync::mpsc, thread, time};
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

#[derive(Debug)]
struct Keybinding {
    combo: String,
    description: String,
    divider: Option<String>,
}
impl Keybinding {
    pub fn ref_array(&self) -> Vec<String> {
        vec![self.combo.to_string(), self.description.to_string()]
    }

    fn combo(&self) -> &str {
        &self.combo
    }
    fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Debug, Default)]
enum KillProcessAction {
    #[default]
    Kill,
    Close,
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    application_mode: ApplicationMode,

    // Search widget
    processes_search_input: String,
    processes_search_input_index: usize,
    processes_search_display: bool,
    // Help Widget
    keybindings: Vec<Keybinding>,
    keybindings_display: bool,
    keybindings_table_state: TableState,
    keybindings_table_scroll_state: ScrollbarState,
    keybindings_table_visible_rows: usize,
    keybindings_table_longest_item_lens: (u16, u16),
    // Kill Widget
    kill_process_display: bool,
    kill_process_item: Option<PortInfo>,
    kill_process_focused_action: KillProcessAction,
    // processes
    processes: Vec<PortInfo>,
    processes_filtered: Vec<PortInfo>,

    // Proccess Table list
    processes_table_state: TableState,
    processes_table_scroll_state: ScrollbarState,
    processes_table_longest_item_lens: (u16, u16, u16, u16, u16),
    processes_table_visible_rows: usize,
    // Theme
    theme_table_colors: TableColors,
    theme_table_color_index: usize,
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
enum ApplicationMode {
    #[default]
    Normal,
    Editing,
    Helping,
    Killing,
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
            processes_search_input: String::new(),
            processes_search_input_index: 0,
            application_mode: ApplicationMode::Normal,
            processes_search_display: false,
            // Help Widget
            keybindings: Self::init_keybindings(),
            keybindings_display: false,
            keybindings_table_state: TableState::default(),
            keybindings_table_scroll_state: ScrollbarState::new((1 * ITEM_HEIGHT) as usize),
            keybindings_table_visible_rows: 0,
            keybindings_table_longest_item_lens: keybindings_constraint_len_calculator(
                &*Self::init_keybindings(),
            ),
            // Kill Widget
            kill_process_display: false,
            kill_process_item: None,
            kill_process_focused_action: KillProcessAction::Kill,
            // Processes
            processes: Vec::new(),
            processes_filtered: Vec::new(),
            // Table list
            processes_table_state: TableState::default(),
            processes_table_scroll_state: ScrollbarState::new((1 * ITEM_HEIGHT) as usize),
            processes_table_longest_item_lens: (5, 5, 30, 55, 5),
            theme_table_colors: TableColors::new(&PALETTES[0]),
            theme_table_color_index: 0,
            processes_table_visible_rows: 0,
        }
    }
    fn init_keybindings() -> Vec<Keybinding> {
        vec![
            Keybinding {
                combo: "Esc / q / Ctrl+C".into(),
                description: "Quit the application".into(),
                divider: None,
            },
            Keybinding {
                combo: "Ctrl+F".into(),
                description: "Toggle the search input".into(),
                divider: None,
            },
            Keybinding {
                combo: "F1 / ?".into(),
                description: "Show or hide this help dialog".into(),
                divider: None,
            },
            Keybinding {
                combo: "j / ↓".into(),
                description: "Move selection down".into(),
                divider: None,
            },
            Keybinding {
                combo: "k / ↑".into(),
                description: "Move selection up".into(),
                divider: None,
            },
            Keybinding {
                combo: "PageDown".into(),
                description: "Page down".into(),
                divider: None,
            },
            Keybinding {
                combo: "PageUp".into(),
                description: "Page up".into(),
                divider: None,
            },
            Keybinding {
                combo: "Shift+PageDown".into(),
                description: "Jump to last item".into(),
                divider: None,
            },
            Keybinding {
                combo: "Shift+PageUp".into(),
                description: "Jump to first item".into(),
                divider: None,
            },
            Keybinding {
                combo: "Shift+Right / l".into(),
                description: "Next color theme".into(),
                divider: None,
            },
            Keybinding {
                combo: "Shift+Left / h".into(),
                description: "Previous color theme".into(),
                divider: None,
            },
            Keybinding {
                combo: "e".into(),
                description: "Enter editing mode".into(),
                divider: None,
            },
        ]
    }

    /// Table list
    pub fn processes_table_next_row(&mut self) {
        let i = match self.processes_table_state.selected() {
            Some(i) => {
                if i >= self.processes_filtered.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.processes_table_state.select(Some(i));
        self.processes_table_scroll_state = self
            .processes_table_scroll_state
            .position(i * ITEM_HEIGHT as usize);
    }
    pub fn processes_table_previous_row(&mut self) {
        let i = match self.processes_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.processes_filtered.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.processes_table_state.select(Some(i));
        self.processes_table_scroll_state = self
            .processes_table_scroll_state
            .position(i * ITEM_HEIGHT as usize);
    }
    pub fn processes_table_go_to_first(&mut self) {
        if !self.processes_filtered.is_empty() {
            self.processes_table_state.select(Some(0));
            self.processes_table_scroll_state = self.processes_table_scroll_state.position(0);
        }
    }
    pub fn processes_table_go_to_last(&mut self) {
        let len = self.processes_filtered.len();
        if len > 0 {
            let last = len - 1;
            self.processes_table_state.select(Some(last));
            self.processes_table_scroll_state = self
                .processes_table_scroll_state
                .position(last * ITEM_HEIGHT as usize);
        }
    }
    pub fn processes_table_page_down(&mut self) {
        let len = self.processes_filtered.len();
        if len == 0 {
            return;
        }

        let current = self.processes_table_state.selected().unwrap_or(0);
        // move down by one screenful, clamped to last row
        let new = (current + self.processes_table_visible_rows).min(len - 1);

        self.processes_table_state.select(Some(new));
        self.processes_table_scroll_state = self
            .processes_table_scroll_state
            .position(new * ITEM_HEIGHT as usize);
    }
    pub fn processes_table_page_up(&mut self) {
        let len = self.processes_filtered.len();
        if len == 0 {
            return;
        }

        let current = self.processes_table_state.selected().unwrap_or(0);
        // move up by one screenful, clamped at zero
        let new = current.saturating_sub(self.processes_table_visible_rows);

        self.processes_table_state.select(Some(new));
        self.processes_table_scroll_state = self
            .processes_table_scroll_state
            .position(new * ITEM_HEIGHT as usize);
    }

    // keybindings
    pub fn keybindings_table_next_row(&mut self) {
        let i = match self.keybindings_table_state.selected() {
            Some(i) => {
                if i >= self.keybindings.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.keybindings_table_state.select(Some(i));
        self.keybindings_table_scroll_state = self
            .keybindings_table_scroll_state
            .position(i * ITEM_HEIGHT as usize);
    }
    pub fn keybindings_table_previous_row(&mut self) {
        let i = match self.keybindings_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.keybindings.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.keybindings_table_state.select(Some(i));
        self.keybindings_table_scroll_state = self
            .keybindings_table_scroll_state
            .position(i * ITEM_HEIGHT as usize);
    }
    pub fn keybindings_table_go_to_first(&mut self) {
        if !self.keybindings.is_empty() {
            self.keybindings_table_state.select(Some(0));
            self.keybindings_table_scroll_state = self.keybindings_table_scroll_state.position(0);
        }
    }
    pub fn keybindings_table_go_to_last(&mut self) {
        let len = self.keybindings.len();
        if len > 0 {
            let last = len - 1;
            self.keybindings_table_state.select(Some(last));
            self.keybindings_table_scroll_state = self
                .keybindings_table_scroll_state
                .position(last * ITEM_HEIGHT as usize);
        }
    }
    pub fn keybindings_table_page_down(&mut self) {
        let len = self.keybindings.len();
        if len == 0 {
            return;
        }

        let current = self.keybindings_table_state.selected().unwrap_or(0);
        // move down by one screenful, clamped to last row
        let new = (current + self.keybindings_table_visible_rows).min(len - 1);

        self.keybindings_table_state.select(Some(new));
        self.keybindings_table_scroll_state = self
            .keybindings_table_scroll_state
            .position(new * ITEM_HEIGHT as usize);
    }
    pub fn keybindings_table_page_up(&mut self) {
        let len = self.keybindings.len();
        if len == 0 {
            return;
        }

        let current = self.keybindings_table_state.selected().unwrap_or(0);
        // move up by one screenful, clamped at zero
        let new = current.saturating_sub(self.keybindings_table_visible_rows);

        self.keybindings_table_state.select(Some(new));
        self.keybindings_table_scroll_state = self
            .keybindings_table_scroll_state
            .position(new * ITEM_HEIGHT as usize);
    }
    pub fn next_color(&mut self) {
        self.theme_table_color_index = (self.theme_table_color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.theme_table_color_index = (self.theme_table_color_index + count - 1) % count;
    }
    pub fn set_colors(&mut self) {
        self.theme_table_colors = TableColors::new(&PALETTES[self.theme_table_color_index]);
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
        let q = self.processes_search_input.to_lowercase();
        self.processes_filtered = self
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
        match self.application_mode {
            ApplicationMode::Normal => self.handle_normal_mode_key(key),
            ApplicationMode::Normal => {
                self.handle_editing_mode_key(key);
                Ok(AppControlFlow::Continue)
            }
            ApplicationMode::Editing => {
                self.handle_editing_mode_key(key);
                Ok(AppControlFlow::Continue)
            }
            ApplicationMode::Helping => {
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
                self.processes_search_display = !self.processes_search_display;
                self.clear_input();

                if self.processes_search_display {
                    self.application_mode = ApplicationMode::Editing;
                }
            }
            (KeyModifiers::NONE, KeyCode::F(1)) | (_, KeyCode::Char('?')) => {
                self.keybindings_display = !self.keybindings_display;

                if (self.keybindings_display) {
                    self.application_mode = ApplicationMode::Helping;
                } else {
                    self.application_mode = ApplicationMode::Normal;
                }
            }
            // Modify Search input mode
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                self.application_mode = ApplicationMode::Editing;
            }
            // Navigate in the list
            (KeyModifiers::SHIFT, KeyCode::PageUp) => self.processes_table_go_to_first(),
            (KeyModifiers::SHIFT, KeyCode::PageDown) => self.processes_table_go_to_last(),
            (KeyModifiers::NONE, KeyCode::PageUp) => self.processes_table_page_up(),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.processes_table_page_down(),
            (KeyModifiers::NONE, KeyCode::Down) => self.processes_table_next_row(),
            (KeyModifiers::NONE, KeyCode::Up) => self.processes_table_previous_row(),
            // Table actions
            (KeyModifiers::NONE, KeyCode::Char('k'))
                if self.processes_table_state.selected().is_some() =>
            {
                self.kill_process_display = !self.kill_process_display;

                if let Some(idx) = self.processes_table_state.selected() {
                    // assuming kill_process_item implements Clone (or Copy),
                    // otherwise use a reference
                    self.kill_process_item = Option::from(self.processes_filtered[idx].clone());
                }
            }
            // Change theme
            (KeyModifiers::SHIFT, KeyCode::Right) => self.next_color(),
            (KeyModifiers::SHIFT, KeyCode::Left) => {
                self.previous_color();
            }
            _ => {}
        }
        Ok(AppControlFlow::Continue)
    }

    fn handle_helping_mode_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc)
            | (KeyModifiers::NONE, KeyCode::F(1))
            | (_, KeyCode::Char('?')) => {
                self.keybindings_display = !self.keybindings_display;
                if (self.keybindings_display) {
                    self.application_mode = ApplicationMode::Helping;
                } else {
                    self.application_mode = ApplicationMode::Normal;
                }
            }
            // Navigate in the list
            (KeyModifiers::SHIFT, KeyCode::PageUp) => self.keybindings_table_go_to_first(),
            (KeyModifiers::SHIFT, KeyCode::PageDown) => self.keybindings_table_go_to_last(),
            (KeyModifiers::NONE, KeyCode::PageUp) => self.keybindings_table_page_up(),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.keybindings_table_page_down(),
            (KeyModifiers::NONE, KeyCode::Down) => self.keybindings_table_next_row(),
            (KeyModifiers::NONE, KeyCode::Up) => self.keybindings_table_previous_row(),

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
                self.application_mode = ApplicationMode::Normal;
                self.processes_table_next_row()
            }
            KeyCode::Up => {
                self.application_mode = ApplicationMode::Normal;
                self.processes_table_previous_row()
            }
            KeyCode::Esc => {
                self.application_mode = ApplicationMode::Normal;
                self.processes_search_display = !self.processes_search_display;

                self.clear_input();

                if self.processes_search_display {
                    self.application_mode = ApplicationMode::Editing;
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

        if !self.processes_search_display {
            let [table_area] = Layout::vertical([Constraint::Min(1)]).areas(area);
            self.processes_table_visible_rows = table_area.height as usize - 1;
            self.render_table(frame, table_area);
            self.render_scrollbar(frame, table_area);
        } else {
            let [input_area, table_area] =
                Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(area);

            self.processes_table_visible_rows = table_area.height as usize - 1;

            self.render_search(frame, input_area);
            self.render_table(frame, table_area);
            self.render_scrollbar(frame, table_area);
        }

        self.render_keybindings_popup(frame, area);
        self.render_kill_popup(frame, area);
    }
    /// helper function to create a centered rect using up certain percentage of the available rect `r`
    fn popup_area(&self, area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Ratio(4, 9)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Ratio(5, 9)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }
    fn kill_prompt_line(&self) -> Line {
        if let Some(item) = &self.kill_process_item {
            let title = format!(
                "Kill {} {:?} port {} ?",
                item.process_name, item.port_state, item.port,
            );
            Line::from(Span::raw(title))
        } else {
            Line::from(Span::raw("Kill ?"))
        }
    }
    fn kill_prompt_description(&self) -> Line {
        if let Some(item) = &self.kill_process_item {
            let s = format!(
                "Ending this process may disrupt services using port {}. Proceeding could result in data loss, network issues, or system instability.",
                item.port,
            );
            Line::from(Span::raw(s))
        } else {
            Line::from(Span::raw("Kill ?"))
        }
    }
    fn render_kill_popup(&mut self, frame: &mut Frame, area: Rect) {
        if self.kill_process_display {
            let block = Block::bordered()
                .border_type(BorderType::Plain)
                .border_style(Style::new().fg(self.theme_table_colors.footer_border_color))
                .bg(self.theme_table_colors.buffer_bg)
                .title("Kill");

            let area = self.popup_area(area, 30, 30);
            frame.render_widget(Clear, area);
            frame.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(2), // spacer
                        Constraint::Length(3), // message
                        Constraint::Length(3), // message
                        Constraint::Min(1),    // spacer
                        Constraint::Length(3), // buttons
                        Constraint::Length(1), // spacer
                    ]
                    .as_ref(),
                )
                .split(area);

            let prompt = Paragraph::new(self.kill_prompt_line())
                .style(
                    Style::default()
                        .fg(self.theme_table_colors.row_fg)
                        .bg(self.theme_table_colors.buffer_bg),
                )
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(Wrap { trim: true });
            let prompt_area = chunks[1].inner(Margin {
                vertical: 0,
                horizontal: 2,
            });
            frame.render_widget(prompt, prompt_area);
            let prompt_description = Paragraph::new(self.kill_prompt_description())
                .style(
                    Style::default()
                        .fg(self.theme_table_colors.row_fg)
                        .bg(self.theme_table_colors.buffer_bg),
                )
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(Wrap { trim: true });
            let prompt_description_area = chunks[2].inner(Margin {
                vertical: 0,
                horizontal: 2,
            });
            frame.render_widget(prompt_description, prompt_description_area);

            let buttons = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
                .flex(Flex::Center)
                .split(chunks[4]);

            //

            let kill_button = Paragraph::new("Kill")
                .alignment(ratatui::layout::Alignment::Center)
                .block(match self.kill_process_focused_action {
                    KillProcessAction::Kill => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(tailwind::RED.c400)),
                    KillProcessAction::Close => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(self.theme_table_colors.footer_border_color)),
                });
            let cancel_button = Paragraph::new("Cancel")
                .alignment(ratatui::layout::Alignment::Center)
                .block(match self.kill_process_focused_action {
                    KillProcessAction::Close => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(tailwind::RED.c400)),
                    KillProcessAction::Kill => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(self.theme_table_colors.footer_border_color)),
                });
            frame.render_widget(kill_button, buttons[0]);
            frame.render_widget(cancel_button, buttons[1]);
        }
    }
    fn render_keybindings_popup(&mut self, frame: &mut Frame, area: Rect) {
        if self.keybindings_display {
            let selected_row_style = Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(self.theme_table_colors.selected_row_style_fg);
            let selected_cell_style = Style::default()
                .add_modifier(Modifier::REVERSED)
                .fg(self.theme_table_colors.selected_cell_style_fg);

            let combo_style = Style::new()
                .fg(self.theme_table_colors.selected_row_style_fg)
                .bg(self.theme_table_colors.buffer_bg);
            let desc_style = Style::new()
                .fg(self.theme_table_colors.row_fg)
                .bg(self.theme_table_colors.buffer_bg);

            let rows = self.keybindings.iter().map(|kb| {
                // build each row by styling its cells individually
                let cells = kb
                    .ref_array()
                    .into_iter()
                    .enumerate()
                    .map(|(col_idx, content)| {
                        let cell = Cell::from(Text::from(content));
                        if col_idx == 0 {
                            cell.style(combo_style)
                        } else {
                            cell.style(desc_style)
                        }
                    });
                Row::new(cells).height(ITEM_HEIGHT)
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Length(self.keybindings_table_longest_item_lens.0 + 1),
                    Constraint::Min(self.keybindings_table_longest_item_lens.1),
                ],
            )
            .row_highlight_style(selected_row_style)
            .cell_highlight_style(selected_cell_style)
            .bg(self.theme_table_colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always)
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.theme_table_colors.footer_border_color))
                    .title("Keybindings"),
            );

            let area = self.popup_area(area, 60, 40);
            self.keybindings_table_visible_rows = area.height as usize - 1;
            frame.render_widget(Clear, area);
            frame.render_stateful_widget(table, area, &mut self.keybindings_table_state);

            frame.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
                &mut self.keybindings_table_scroll_state,
            );
        }
    }

    fn render_search(&mut self, frame: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.processes_search_input.as_str())
            .style(
                Style::default()
                    .fg(self.theme_table_colors.row_fg)
                    .bg(self.theme_table_colors.buffer_bg),
            )
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.theme_table_colors.footer_border_color))
                    .title("Search"),
            );

        frame.render_widget(input, area);

        match self.application_mode {
            #[allow(clippy::cast_possible_truncation)]
            ApplicationMode::Editing => frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                area.x + self.processes_search_input_index as u16 + 1,
                // Move one line down, from the border to the input line
                area.y + 1,
            )),
            _ => {}
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(self.theme_table_colors.header_fg)
            .bg(self.theme_table_colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.theme_table_colors.selected_row_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.theme_table_colors.selected_cell_style_fg);

        let header = ["PID", "Port", "Process Name", "Process Path", "Listener"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.processes_filtered.iter().enumerate().map(|(i, data)| {
            // let color = match i % 2 {
            //     0 => self.processes_table_colors.normal_row_color,
            //     _ => self.processes_table_colors.alt_row_color,
            // };
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
                Constraint::Length(self.processes_table_longest_item_lens.0),
                Constraint::Min(self.processes_table_longest_item_lens.1),
                Constraint::Min(self.processes_table_longest_item_lens.2),
                Constraint::Min(self.processes_table_longest_item_lens.3),
                Constraint::Min(self.processes_table_longest_item_lens.4),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .cell_highlight_style(selected_cell_style)
        .bg(self.theme_table_colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);

        // .block(
        //     Block::bordered()
        //         .border_type(BorderType::Plain)
        //         .border_style(Style::new().fg(self.colors.footer_border_color)),
        // )

        frame.render_stateful_widget(t, area, &mut self.processes_table_state);
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
            &mut self.processes_table_scroll_state,
        );
    }
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.theme_table_colors.row_fg)
                    .bg(self.theme_table_colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Plain)
                    .border_style(Style::new().fg(self.theme_table_colors.footer_border_color)),
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
        self.processes_search_input.clear();
        self.processes_search_input_index = 0;
        self.update_filtered_processes();
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.processes_search_input.chars().count())
    }
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.processes_search_input_index.saturating_sub(1);
        self.processes_search_input_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.processes_search_input_index.saturating_add(1);
        self.processes_search_input_index = self.clamp_cursor(cursor_moved_right);
    }

    fn byte_index(&self) -> usize {
        self.processes_search_input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.processes_search_input_index)
            .unwrap_or(self.processes_search_input.len())
    }
    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.processes_search_input.insert(index, new_char);
        self.move_cursor_right();
        self.update_filtered_processes();
    }
    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.processes_search_input_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.processes_search_input_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self
                .processes_search_input
                .chars()
                .take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.processes_search_input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.processes_search_input =
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
                let length = self.processes_filtered.len() * ITEM_HEIGHT as usize;
                self.processes_table_scroll_state =
                    self.processes_table_scroll_state.content_length(length);
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

fn keybindings_constraint_len_calculator(items: &[Keybinding]) -> (u16, u16) {
    let combo = items
        .iter()
        .map(Keybinding::combo)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let description = items
        .iter()
        .map(Keybinding::description)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (combo as u16, description as u16)
}
