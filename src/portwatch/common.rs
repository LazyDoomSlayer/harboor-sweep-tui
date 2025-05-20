use crate::model::PortInfo;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Supported export formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Csv,
    Json,
    Yaml,
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
