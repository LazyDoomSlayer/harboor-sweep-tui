use crate::model::PortInfo;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Supported export formats
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
    Yaml,
}

impl ExportFormat {
    pub fn next(self) -> Self {
        match self {
            ExportFormat::Json => ExportFormat::Csv,
            ExportFormat::Csv => ExportFormat::Yaml,
            ExportFormat::Yaml => ExportFormat::Json,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            ExportFormat::Json => ExportFormat::Yaml,
            ExportFormat::Csv => ExportFormat::Json,
            ExportFormat::Yaml => ExportFormat::Csv,
        }
    }
}

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
