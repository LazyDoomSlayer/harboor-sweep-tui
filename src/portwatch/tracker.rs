use crate::model::PortInfo;

use chrono::{DateTime, Utc};
use csv::Writer;

use crate::portwatch::{ExportFormat, common::PortEvent, export::export_to_file};
use std::{
    collections::HashSet,
    io::{Result, Write},
    path::PathBuf,
};

#[derive(Debug, Default)]
pub struct Tracker {
    pub events: Vec<PortEvent>,
    pub baseline: Vec<PortInfo>,
    pub started_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub export_format: ExportFormat,
}

impl Tracker {
    pub fn new() -> Self {
        Self {
            events: vec![],
            baseline: vec![],
            started_at: None,
            is_active: false,
            export_format: ExportFormat::Json,
        }
    }

    /// Starts the tracker and takes a baseline snapshot of current ports.
    pub fn start(&mut self, current_ports: Vec<PortInfo>) {
        self.started_at = Some(Utc::now());
        self.is_active = true;
        self.events.clear();
        self.baseline = current_ports.clone();
        self.events.push(PortEvent::InitialState {
            timestamp: Utc::now(),
            ports: current_ports,
        });
    }

    /// Stops the tracker and immediately exports all collected events as JSON.
    pub fn stop(&mut self) {
        self.is_active = false;
        match self.export(None) {
            _ => {}
        }
    }

    /// Tracks differences between the baseline and current state.
    pub fn track_once(&mut self, current_ports: Vec<PortInfo>) {
        if !self.is_active {
            return;
        }

        let (added, removed) = Self::diff_ports(&self.baseline, &current_ports);

        for port in added {
            self.events.push(PortEvent::PortOpened {
                timestamp: Utc::now(),
                port,
            });
        }

        for port in removed {
            self.events.push(PortEvent::PortClosed {
                timestamp: Utc::now(),
                port,
            });
        }

        self.baseline = current_ports;
    }

    pub fn export(&self, output_dir: Option<&PathBuf>) -> Result<PathBuf> {
        export_to_file(
            &self.events,
            self.export_format,
            "changes",
            output_dir,
            Some(Self::write_events_csv),
        )
    }

    fn write_events_csv(file: &mut dyn Write, events: &[PortEvent]) -> Result<()> {
        let mut wtr = Writer::from_writer(file);
        wtr.write_record(&[
            "timestamp",
            "event",
            "port",
            "pid",
            "process_name",
            "process_path",
        ])?;

        for event in events {
            match event {
                PortEvent::InitialState { timestamp, ports } => {
                    for p in ports {
                        wtr.write_record(&[
                            timestamp.to_rfc3339(),
                            "initial_state".parse().unwrap(),
                            p.port.to_string(),
                            p.pid.to_string(),
                            p.process_name.clone(),
                            p.process_path.clone(),
                        ])?;
                    }
                }
                PortEvent::PortOpened { timestamp, port } => {
                    wtr.write_record(&[
                        timestamp.to_rfc3339(),
                        "port_opened".parse().unwrap(),
                        port.port.to_string(),
                        port.pid.to_string(),
                        port.process_name.clone(),
                        port.process_path.clone(),
                    ])?;
                }
                PortEvent::PortClosed { timestamp, port } => {
                    wtr.write_record(&[
                        timestamp.to_rfc3339(),
                        "port_closed".parse().unwrap(),
                        port.port.to_string(),
                        port.pid.to_string(),
                        port.process_name.clone(),
                        port.process_path.clone(),
                    ])?;
                }
            }
        }

        wtr.flush()
    }

    /// Internal helper to compute diff between two sets of ports.
    fn diff_ports(old: &[PortInfo], new: &[PortInfo]) -> (Vec<PortInfo>, Vec<PortInfo>) {
        let old_set: HashSet<_> = old.iter().cloned().collect();
        let new_set: HashSet<_> = new.iter().cloned().collect();

        let added = new_set.difference(&old_set).cloned().collect();
        let removed = old_set.difference(&new_set).cloned().collect();

        (added, removed)
    }
}
