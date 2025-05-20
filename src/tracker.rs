use crate::PortInfo;
use crate::explorer::ExportFormat;

use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "event")]
pub enum PortEvent {
    #[serde(rename = "initial_state")]
    InitialState {
        timestamp: DateTime<Utc>,
        ports: Vec<PortInfo>,
    },
    #[serde(rename = "port_opened")]
    PortOpened {
        timestamp: DateTime<Utc>,
        port: PortInfo,
    },
    #[serde(rename = "port_closed")]
    PortClosed {
        timestamp: DateTime<Utc>,
        port: PortInfo,
    },
}

#[derive(Debug, Default)]
pub struct Tracker {
    pub events: Vec<PortEvent>,
    pub baseline: Vec<PortInfo>,
    pub started_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl Tracker {
    pub fn new() -> Self {
        Self {
            events: vec![],
            baseline: vec![],
            started_at: None,
            is_active: false,
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
        match self.export(ExportFormat::Json, None) {
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

    /// Exports all recorded port events to a file in the given format.
    pub fn export(
        &self,
        format: ExportFormat,
        output_dir: Option<&PathBuf>,
    ) -> io::Result<PathBuf> {
        let base_dir = output_dir.cloned().unwrap_or_else(|| PathBuf::from("."));
        let snapshots_dir = base_dir.join("snapshots");
        std::fs::create_dir_all(&snapshots_dir)?;

        let ts = Local::now().format("%Y%m%d-%H%M%S").to_string();
        let file_name = match format {
            ExportFormat::Csv => format!("changes-{}.csv", ts),
            ExportFormat::Json => format!("changes-{}.json", ts),
            ExportFormat::Yaml => format!("changes-{}.yaml", ts),
        };

        let path = snapshots_dir.join(file_name);
        let mut file = File::create(&path)?;

        match format {
            ExportFormat::Csv => self.write_csv(&mut file),
            ExportFormat::Json => self.write_json(&mut file),
            ExportFormat::Yaml => self.write_yaml(&mut file),
        }?;

        Ok(path)
    }

    /// Internal helper to compute diff between two sets of ports.
    fn diff_ports(old: &[PortInfo], new: &[PortInfo]) -> (Vec<PortInfo>, Vec<PortInfo>) {
        let old_set: HashSet<_> = old.iter().cloned().collect();
        let new_set: HashSet<_> = new.iter().cloned().collect();

        let added = new_set.difference(&old_set).cloned().collect();
        let removed = old_set.difference(&new_set).cloned().collect();

        (added, removed)
    }

    fn write_json(&self, file: &mut impl Write) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.events)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        file.write_all(json.as_bytes())
    }

    fn write_yaml(&self, file: &mut impl Write) -> io::Result<()> {
        let yaml = serde_yaml::to_string(&self.events)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        file.write_all(yaml.as_bytes())
    }

    fn write_csv(&self, file: &mut impl Write) -> io::Result<()> {
        let mut wtr = csv::Writer::from_writer(file);
        wtr.write_record(&[
            "timestamp",
            "event",
            "port",
            "pid",
            "process_name",
            "process_path",
        ])?;

        for event in &self.events {
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
}
