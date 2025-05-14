use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{self, Write};
use std::path::PathBuf;

use crate::explorer::{export_snapshot, ExportFormat};
use crate::model::PortInfo;

#[derive(Debug, Clone)]
pub struct PortChange {
    pub added: Vec<PortInfo>,
    pub removed: Vec<PortInfo>,
    pub started_at: DateTime<Local>,
    pub exported_at: Option<DateTime<Local>>,
}

impl PortChange {
    pub fn new() -> Self {
        Self {
            added: vec![],
            removed: vec![],
            started_at: Local::now(),
            exported_at: None,
        }
    }

    pub fn detect_changes(&mut self, previous: &[PortInfo], current: &[PortInfo]) {
        let old_map: HashMap<u16, &PortInfo> = previous.iter().map(|p| (p.port, p)).collect();
        let new_map: HashMap<u16, &PortInfo> = current.iter().map(|p| (p.port, p)).collect();

        self.added = new_map
            .iter()
            .filter(|(port, _)| !old_map.contains_key(port))
            .map(|(_, p)| (*p).clone())
            .collect();

        self.removed = old_map
            .iter()
            .filter(|(port, _)| !new_map.contains_key(port))
            .map(|(_, p)| (*p).clone())
            .collect();
    }

    pub fn export_to_file(
        &mut self,
        format: ExportFormat,
        output_dir: Option<&PathBuf>,
    ) -> io::Result<PathBuf> {
        self.exported_at = Some(Local::now());

        // generate metadata line
        let meta_line = format!(
            "# Snapshot started: {} | Exported: {}\n",
            self.started_at.format("%Y-%m-%d %H:%M:%S"),
            self.exported_at.unwrap().format("%Y-%m-%d %H:%M:%S")
        );

        let combined: Vec<PortInfo> = self
            .added
            .iter()
            .cloned()
            .chain(self.removed.iter().cloned())
            .collect();

        let ts = self
            .exported_at
            .unwrap()
            .format("%Y%m%d-%H%M%S")
            .to_string();
        let suffix = match format {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Yaml => "yaml",
        };
        let file_name = format!(
            "changes-{}-{}.{}",
            self.started_at.format("%H%M%S"),
            ts,
            suffix
        );

        let mut path = output_dir
            .cloned()
            .unwrap_or_else(|| PathBuf::from("snapshot"));
        create_dir_all(&path)?;
        path.push(file_name);

        let mut file = File::create(&path)?;

        // writing metadata
        file.write_all(meta_line.as_bytes())?;

        match format {
            ExportFormat::Csv => export_snapshot(
                &combined,
                ExportFormat::Csv,
                Some(&path.parent().unwrap().to_path_buf()),
            ),
            ExportFormat::Json => export_snapshot(
                &combined,
                ExportFormat::Json,
                Some(&path.parent().unwrap().to_path_buf()),
            ),
            ExportFormat::Yaml => export_snapshot(
                &combined,
                ExportFormat::Yaml,
                Some(&path.parent().unwrap().to_path_buf()),
            ),
        }?;

        Ok(path)
    }
}
