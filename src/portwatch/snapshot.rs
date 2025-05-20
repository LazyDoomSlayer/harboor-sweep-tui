use crate::model::PortInfo;

use crate::portwatch::{ExportFormat, export::export_to_file};

use csv::Writer;
use std::{
    io::{Result, Write},
    path::PathBuf,
};

pub fn export_snapshot(
    entries: &[PortInfo],
    format: ExportFormat,
    output_dir: Option<&PathBuf>,
) -> Result<PathBuf> {
    export_to_file(
        entries,
        format,
        "ports",
        output_dir,
        Some(write_snapshot_csv),
    )
}

fn write_snapshot_csv(file: &mut dyn Write, entries: &[PortInfo]) -> Result<()> {
    let mut wtr = Writer::from_writer(file);
    wtr.write_record(&["Port", "PID", "Process Name", "Process Path", "State"])?;
    for p in entries {
        wtr.write_record(&[
            p.port.to_string(),
            p.pid.to_string(),
            p.process_name.clone(),
            p.process_path.clone(),
            format!("{:?}", p.port_state),
        ])?;
    }
    wtr.flush()
}
