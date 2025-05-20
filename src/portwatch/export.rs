use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

use crate::portwatch::ExportFormat;
use chrono::Local;
use serde::Serialize;

/// Writes any serializable entries to a JSON/YAML/CSV file under the `/snapshots` folder.
pub fn export_to_file<T: Serialize>(
    data: &[T],
    format: ExportFormat,
    file_prefix: &str,
    output_dir: Option<&PathBuf>,
    write_csv_fn: Option<fn(&mut dyn Write, &[T]) -> io::Result<()>>,
) -> io::Result<PathBuf> {
    let base_dir = output_dir.cloned().unwrap_or_else(|| PathBuf::from("."));
    let snapshots_dir = base_dir.join("snapshots");
    std::fs::create_dir_all(&snapshots_dir)?;

    let ts = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let ext = match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Json => "json",
        ExportFormat::Yaml => "yaml",
    };
    let filename = format!("{file_prefix}-{ts}.{ext}");
    let path = snapshots_dir.join(filename);
    let mut file = File::create(&path)?;

    match format {
        ExportFormat::Csv => {
            if let Some(write_fn) = write_csv_fn {
                write_fn(&mut file, data)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "CSV writer not provided",
                ));
            }
        }
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(data)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            file.write_all(json.as_bytes())?;
        }
        ExportFormat::Yaml => {
            let yaml =
                serde_yaml::to_string(data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            file.write_all(yaml.as_bytes())?;
        }
    }

    Ok(path)
}
