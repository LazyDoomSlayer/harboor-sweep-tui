mod event_tracker;
mod explorer;
mod model;
mod ui;
mod util;

use crate::explorer::{ExportFormat, export_snapshot};
use crate::model::{PortInfo, os};
use crate::ui::{
    footer_component::FooterComponent,
    keybindings_component::KeybindingsComponent,
    kill_process_component::{KillAction, KillComponent},
    process_search_component::ProcessSearchComponent,
    process_table_component::ProcessTableComponent,
    process_table_component::SortBy,
    snapshots_component::{ExportAction, SnapshotsComponent},
    theme::Theme,
};

use color_eyre::Result;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    layout::{Constraint, Direction, Layout},
};

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
    pub snapshots_component: SnapshotsComponent,
    pub footer_component: FooterComponent,

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
    Snapshotting,
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
            snapshots_component: SnapshotsComponent::default(),
            footer_component: FooterComponent::default(),

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
    /// Render the application's UI.
    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let mut layout_constraints = vec![];

        if self.search.display {
            layout_constraints.push(Constraint::Length(3));
        }

        layout_constraints.push(Constraint::Min(1));

        if self.footer_component.display {
            layout_constraints.push(Constraint::Length(3));
        }

        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(layout_constraints)
            .split(area);

        let mut index = 0;

        if self.search.display {
            let input_area = areas[index];
            self.search
                .render(frame, input_area, &self.theme.table, &self.application_mode);
            index += 1;
        }

        let table_area = areas[index];
        self.table.visible_rows = table_area.height as usize - 1;
        self.table.render(frame, table_area, &self.theme.table);
        index += 1;

        if self.footer_component.display {
            let footer_area = areas[index];
            self.footer_component
                .render(frame, footer_area, &self.theme.table);
        }

        // Popups
        self.keybindings.render(frame, area, &self.theme.table);
        self.kill_process.render(frame, area, &self.theme.table);
        self.snapshots_component
            .render(frame, area, &self.theme.table);
    }

    /// Toggles the processes search display.
    fn toggle_processes_search_display(&mut self) {
        self.search.toggle();
        self.update_filtered_processes();

        if self.search.display {
            self.application_mode = ApplicationMode::Editing;
        } else {
            self.application_mode = ApplicationMode::Normal;
        }
    }
    /// Toggles the keybindings display.
    fn toggle_keybindings_display(&mut self) {
        self.keybindings.display = !self.keybindings.display;

        if self.keybindings.display {
            self.application_mode = ApplicationMode::Helping;
        } else {
            self.application_mode = ApplicationMode::Normal;
        }
    }
    /// Toggles the snapshotting display.
    fn toggle_snapshotting_display(&mut self) {
        self.snapshots_component.toggle();

        if self.snapshots_component.display {
            self.application_mode = ApplicationMode::Snapshotting;
        } else {
            self.application_mode = ApplicationMode::Normal;
        }
    }

    /// User input controller handling different modes.
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
            ApplicationMode::Snapshotting => {
                self.handle_snapshotting_mode_key(key);
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
            (KeyModifiers::NONE, KeyCode::F(2)) => self.toggle_snapshotting_display(),
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
            // Change sorting in table
            (KeyModifiers::NONE, KeyCode::Char('1')) => self.table.set_or_toggle_sort(SortBy::Port),
            (KeyModifiers::NONE, KeyCode::Char('2')) => self.table.set_or_toggle_sort(SortBy::PID),
            (KeyModifiers::NONE, KeyCode::Char('3')) => {
                self.table.set_or_toggle_sort(SortBy::ProcessName)
            }
            (KeyModifiers::NONE, KeyCode::Char('4')) => {
                self.table.set_or_toggle_sort(SortBy::ProcessPath)
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
    fn handle_snapshotting_mode_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::F(2)) => {
                self.toggle_snapshotting_display()
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                self.snapshots_component.action = ExportAction::Export;
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.snapshots_component.action = ExportAction::Cancel;
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.snapshots_component.next_format();
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.snapshots_component.prev_format();
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                match self.kill_process.action {
                    KillAction::Kill => {
                        let entries = self.table.items.clone();
                        let export_type = self.snapshots_component.selected_format.clone();
                        thread::spawn(move || {
                            let _ = export_snapshot(&entries, export_type, None);
                        });
                    }
                    KillAction::Cancel => {}
                }
                self.toggle_snapshotting_display();
            }

            _ => {}
        }
    }
    /// Monitors the ports and updates the processes list.
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

    /// Filters ports and updates filtered list.
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
}
