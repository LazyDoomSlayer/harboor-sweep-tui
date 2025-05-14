mod event_tracker;
mod explorer;
mod model;
mod ui;
mod util;

use crate::model::{PortInfo, os};
use crate::ui::{
    keybindings_component::KeybindingsComponent,
    kill_process_component::{KillAction, KillComponent},
    process_search_component::ProcessSearchComponent,
    process_table_component::ProcessTableComponent,
    theme::Theme,
};

use crate::util::popup_area;

use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Direction, Flex, Layout, Margin, Rect},
    prelude::Style,
    style::{Stylize, palette::tailwind},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
};

use crate::explorer::{ExportFormat, export_snapshot};
use std::{sync::mpsc, thread, time};

const ITEM_HEIGHT: u16 = 1;

fn bootstrap() -> Result<()> {
    let (event_tx, event_rx) = mpsc::channel::<MultithreadingEvent>();
    let tx_to_input_events = event_tx.clone();
    let tx_to_background_thread = event_tx.clone();

    thread::spawn(move || {
        handle_input_events(tx_to_input_events);
    });
    thread::spawn(move || {
        run_background_thread(tx_to_background_thread);
    });

    let terminal = ratatui::init();
    let result = App::new().run(terminal, event_rx);

    ratatui::restore();
    result
}
fn main() -> Result<()> {
    color_eyre::install()?;
    bootstrap()
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    pub application_mode: ApplicationMode,

    // Search component
    pub search: ProcessSearchComponent,
    pub table: ProcessTableComponent,
    pub keybindings: KeybindingsComponent,
    pub theme: Theme,
    pub kill_process: KillComponent,

    // processes
    processes: Vec<PortInfo>,
    processes_filtered: Vec<PortInfo>,
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

#[derive(Debug, Default)]
pub enum ApplicationMode {
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
            application_mode: ApplicationMode::Normal,

            search: ProcessSearchComponent::default(),
            table: ProcessTableComponent::default(),
            keybindings: KeybindingsComponent::default(),
            theme: Theme::default(),
            kill_process: KillComponent::default(),

            // Processes
            processes: Vec::new(),
            processes_filtered: Vec::new(),
        }
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
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        if !self.search.display {
            let [table_area] = Layout::vertical([Constraint::Min(1)]).areas(area);
            self.table.visible_rows = table_area.height as usize - 1;
            self.table.render(frame, table_area, &self.theme.table);
            self.render_scrollbar(frame, table_area);
        } else {
            let [input_area, table_area] =
                Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(area);

            self.table.visible_rows = table_area.height as usize - 1;

            self.search
                .render(frame, input_area, &self.theme.table, &self.application_mode);
            self.table.render(frame, table_area, &self.theme.table);
            self.render_scrollbar(frame, table_area);
        }

        if self.keybindings.display {
            self.keybindings.render(frame, area, &self.theme.table);
        }
        self.render_kill_popup(frame, area);
    }

    fn update_filtered_processes(&mut self) {
        let q = self.search.value.to_lowercase();
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

        self.table.set_items(self.processes_filtered.clone());
    }

    fn toggle_processes_search_display(&mut self) {
        self.search.toggle();
        self.update_filtered_processes();

        if self.search.display {
            self.application_mode = ApplicationMode::Editing;
        } else {
            self.application_mode = ApplicationMode::Normal;
        }
    }
    fn toggle_keybindings_display(&mut self) {
        self.keybindings.display = !self.keybindings.display;

        if self.keybindings.display {
            self.application_mode = ApplicationMode::Helping;
        } else {
            self.application_mode = ApplicationMode::Normal;
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<AppControlFlow> {
        match self.application_mode {
            ApplicationMode::Normal => self.handle_normal_mode_key(key),
            ApplicationMode::Killing => {
                self.handle_killing_mode_key(key);
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
                self.toggle_processes_search_display()
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x' | 'X')) => {
                let entries = self.table.items.clone();
                thread::spawn(move || {
                    let _ = export_snapshot(&entries, ExportFormat::Json, None);
                });
            }
            // (KeyModifiers::CONTROL, KeyCode::Char('x' | 'X')) => {
            //     let entries = self.table.items.clone();
            //     let metadata = crate::explorer::ExportMetadata {
            //         started_at: self.start_time,
            //         exported_at: chrono::Local::now(),
            //     };
            //     thread::spawn(move || {
            //         let _ = ExportFormat::Json.export_snapshot_with_metadata(&entries, None, Some(metadata));
            //     });
            // }
            (KeyModifiers::NONE, KeyCode::F(1)) | (_, KeyCode::Char('?')) => {
                self.toggle_keybindings_display();
            }
            // Modify Search input mode
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                self.application_mode = ApplicationMode::Editing;
            }
            // Navigate in the list
            (KeyModifiers::SHIFT, KeyCode::PageUp) => self.table.first_row(),
            (KeyModifiers::SHIFT, KeyCode::PageDown) => self.table.last_row(),
            (KeyModifiers::NONE, KeyCode::PageUp) => self.table.page_up(),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.table.page_down(),
            (KeyModifiers::NONE, KeyCode::Down) => self.table.next_row(),
            (KeyModifiers::NONE, KeyCode::Up) => self.table.previous_row(),
            // Table actions
            (KeyModifiers::NONE, KeyCode::Char('k')) if self.table.state.selected().is_some() => {
                self.kill_process.display = !self.kill_process.display;
                if self.kill_process.display {
                    self.application_mode = ApplicationMode::Killing;
                } else {
                    self.application_mode = ApplicationMode::Normal;
                }

                if let Some(idx) = self.table.state.selected() {
                    // assuming kill_process.item implements Clone (or Copy),
                    // otherwise use a reference
                    self.kill_process.item = Option::from(self.processes_filtered[idx].clone());
                }
            }
            // Change theme
            (KeyModifiers::SHIFT, KeyCode::Right) => self.theme.cycle_next(),
            (KeyModifiers::SHIFT, KeyCode::Left) => {
                self.theme.cycle_prev();
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
                self.toggle_keybindings_display();
            }
            // Navigate in the list
            (KeyModifiers::SHIFT, KeyCode::PageUp) => self.keybindings.first_row(),
            (KeyModifiers::SHIFT, KeyCode::PageDown) => self.keybindings.last_row(),
            (KeyModifiers::NONE, KeyCode::PageUp) => self.keybindings.page_up(),
            (KeyModifiers::NONE, KeyCode::PageDown) => self.keybindings.page_down(),
            (KeyModifiers::NONE, KeyCode::Down) => self.keybindings.next_row(),
            (KeyModifiers::NONE, KeyCode::Up) => self.keybindings.previous_row(),

            _ => {}
        }
    }
    fn handle_editing_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(to_insert) => {
                self.search.insert_char(to_insert);
                self.update_filtered_processes();
            }
            KeyCode::Backspace => {
                self.search.delete_char();
                self.update_filtered_processes();
            }
            KeyCode::Left => self.search.move_cursor_left(),
            KeyCode::Right => self.search.move_cursor_right(),
            KeyCode::Down => {
                self.application_mode = ApplicationMode::Normal;
                self.table.next_row()
            }
            KeyCode::Up => {
                self.application_mode = ApplicationMode::Normal;
                self.table.previous_row()
            }
            KeyCode::Esc => self.toggle_processes_search_display(),

            _ => {}
        }
    }
    fn handle_killing_mode_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Left) => {
                self.kill_process.action = KillAction::Kill;
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.kill_process.action = KillAction::Cancel;
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                match self.kill_process.action {
                    KillAction::Kill => {
                        if let Some(item) = self.kill_process.item.take() {
                            let killing_response = os::kill_process(item.pid);
                            if killing_response.success {
                                self.processes.retain(|p| p.pid != item.pid);
                                self.update_filtered_processes();
                            }
                        }
                    }
                    KillAction::Cancel => {
                        self.kill_process.item.take();
                    }
                }
                self.kill_process.display = false;
                self.application_mode = ApplicationMode::Normal;
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.kill_process.display = false;
                self.application_mode = ApplicationMode::Normal;
                self.kill_process.item.take();
            }
            _ => {}
        }
    }

    fn kill_prompt_line(&self) -> Line {
        if let Some(item) = &self.kill_process.item {
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
        if let Some(item) = &self.kill_process.item {
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
        if self.kill_process.display {
            let block = Block::bordered()
                .border_type(BorderType::Plain)
                .border_style(Style::new().fg(self.theme.table.footer_border_color))
                .bg(self.theme.table.buffer_bg)
                .title("Kill");

            let area = popup_area(area, 4, 5);
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
                        .fg(self.theme.table.row_fg)
                        .bg(self.theme.table.buffer_bg),
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
                        .fg(self.theme.table.row_fg)
                        .bg(self.theme.table.buffer_bg),
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

            let kill_button = Paragraph::new("Kill")
                .alignment(ratatui::layout::Alignment::Center)
                .block(match self.kill_process.action {
                    KillAction::Kill => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(tailwind::RED.c400)),
                    KillAction::Cancel => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(tailwind::GRAY.c400)),
                });
            let cancel_button = Paragraph::new("Cancel")
                .alignment(ratatui::layout::Alignment::Center)
                .block(match self.kill_process.action {
                    KillAction::Cancel => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(tailwind::RED.c400)),
                    KillAction::Kill => Block::bordered()
                        .border_type(BorderType::Plain)
                        .border_style(Style::new().fg(tailwind::GRAY.c400)),
                });
            frame.render_widget(kill_button, buttons[0]);
            frame.render_widget(cancel_button, buttons[1]);
        }
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
            &mut self.table.scroll,
        );
    }

    fn monitor_ports_loop(&mut self) {
        match os::fetch_ports() {
            Ok(ports) => {
                self.processes = ports;
                self.update_filtered_processes();
                let length = self.processes_filtered.len() * ITEM_HEIGHT as usize;
                self.table.scroll = self.table.scroll.content_length(length);
            }
            Err(e) => {
                eprintln!("Error fetching ports: {}", e);
            }
        }
    }
}
