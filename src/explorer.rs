use std::fs::File;
use std::io::{self, Write};
use std::path::{PathBuf};
use chrono::Local;
use csv::Writer;
use crate::model::PortInfo;

/// Supported export formats
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Csv,
    Json,
    Yaml,
}

/// Exports a snapshot of PortInfo entries to a file in the given format.
/// Returns the full path of the created file on success.
pub fn export_snapshot(
    entries: &[PortInfo],
    format: ExportFormat,
    output_dir: Option<&PathBuf>,
) -> io::Result<PathBuf> {
    // Determine timestamped filename
    let ts = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let file_name = match format {
        ExportFormat::Csv  => format!("ports-{}.csv", ts),
        ExportFormat::Json => format!("ports-{}.json", ts),
        ExportFormat::Yaml => format!("ports-{}.yaml", ts),
    };

    // Build path: either provided dir or current working directory
    let mut path = output_dir.cloned().unwrap_or_else(|| PathBuf::from("."));
    path.push(file_name);

    // Create file
    let mut file = File::create(&path)?;

    // Dispatch to format-specific writer
    match format {
        ExportFormat::Csv => write_csv(&mut file, entries),
        ExportFormat::Json => write_json(&mut file, entries),
        ExportFormat::Yaml => write_yaml(&mut file, entries),
    }?;

    Ok(path)
}

fn write_csv(file: &mut impl Write, entries: &[PortInfo]) -> io::Result<()> {
    let mut wtr = Writer::from_writer(file);
    // header
    wtr.write_record(&["Port", "PID", "Process Name", "Process Path", "State"])?;
    // rows
    for p in entries {
        wtr.write_record(&[
            p.port.to_string(),
            p.pid.to_string(),
            p.process_name.clone(),
            p.process_path.clone(),
            format!("{:?}", p.port_state),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

fn write_json(file: &mut impl Write, entries: &[PortInfo]) -> io::Result<()> {
    // serialize to JSON
    let json = serde_json::to_string_pretty(entries)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

fn write_yaml(file: &mut impl Write, entries: &[PortInfo]) -> io::Result<()> {
    let yaml = serde_yaml::to_string(entries)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    file.write_all(yaml.as_bytes())?;
    Ok(())
}
