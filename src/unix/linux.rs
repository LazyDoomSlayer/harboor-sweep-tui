use crate::common::{
    KillProcessResponse, PortInfo, ProcessInfo, ProcessInfoResponse, ProcessPortState,
};

use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

pub fn fetch_ports() -> Result<Vec<PortInfo>, String> {
    let output = Command::new("lsof")
        .args(["-i", "-P", "-n"])
        .output()
        .map_err(|e| format!("Failed to execute lsof: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "lsof command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_lsof_output(&stdout)
}

fn parse_lsof_output(output: &str) -> Result<Vec<PortInfo>, String> {
    let mut seen = HashSet::new();
    let mut ports = Vec::new();

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let pid: u32 = match parts[1].parse() {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        let port: u16 = parts[8]
            .split(':')
            .last()
            .unwrap_or("0")
            .parse::<u16>()
            .unwrap_or(0);

        let process_path = match get_process_path(pid) {
            Ok(path) => path,
            Err(err) => err,
        };

        let port_state = if parts.get(9).map_or(false, |state| state.contains("LISTEN")) {
            ProcessPortState::Hosting
        } else {
            ProcessPortState::Using
        };

        if seen.insert((pid, port)) {
            ports.push(PortInfo {
                id: generate_unique_id(pid, port, parts[0]),
                pid,
                process_name: parts[0].to_string(),
                port,
                process_path,
                port_state,
            });
        }
    }

    Ok(ports)
}

fn generate_unique_id(pid: u32, port: u16, process_name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    pid.hash(&mut hasher);
    port.hash(&mut hasher);
    process_name.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn get_process_path(pid: u32) -> Result<String, String> {
    let exe_path = format!("/proc/{}/exe", pid);
    match std::fs::read_link(&exe_path) {
        Ok(path) => Ok(path.to_string_lossy().to_string()),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                Err("Permission Denied".to_string())
            } else if err.kind() == std::io::ErrorKind::NotFound {
                Err("Process not found".to_string())
            } else {
                Err("Unknown error".to_string())
            }
        }
    }
}

pub fn kill_process(pid: u32) -> KillProcessResponse {
    let output = Command::new("kill").arg(pid.to_string()).output();

    match output {
        Ok(output) if output.status.success() => KillProcessResponse {
            success: true,
            message: format!("Successfully killed process with PID {}", pid),
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            KillProcessResponse {
                success: false,
                message: format!(
                    "Failed to kill process {} (Exit code: {}): {}",
                    pid,
                    exit_code,
                    stderr.trim()
                ),
            }
        }
        Err(e) => KillProcessResponse {
            success: false,
            message: format!("Failed to execute kill command: {}", e),
        },
    }
}

pub fn get_processes_using_port(port: u16, item_pid: u32) -> Result<ProcessInfoResponse, String> {
    let output = Command::new("lsof")
        .arg("-i")
        .arg(format!(":{}", port))
        .output()
        .map_err(|e| format!("Failed to execute lsof command: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "lsof command failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();

        if fields.len() < 10 {
            continue;
        }

        let pid: u32 = match fields[1].parse() {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        let address_port = fields[8];
        let state = fields[9];

        if !state.contains("LISTEN") {
            continue;
        }

        let parsed_port: u16 = match address_port.split(':').last().unwrap_or_default().parse() {
            Ok(port) => port,
            Err(_) => continue,
        };

        if parsed_port != port {
            continue;
        }

        if pid == item_pid {
            return Ok(ProcessInfoResponse {
                port_state: ProcessPortState::Hosting,
                data: None,
            });
        }

        if let Some(process_info) = get_process_info(pid, port) {
            return Ok(ProcessInfoResponse {
                port_state: ProcessPortState::Using,
                data: Some(process_info),
            });
        }
    }

    Err(format!("No processes found listening on port {}", port))
}

fn get_process_info(pid: u32, port: u16) -> Option<ProcessInfo> {
    let proc_path = PathBuf::from(format!("/proc/{}/", pid));

    let process_name = fs::read_to_string(proc_path.join("comm"))
        .ok()?
        .trim()
        .to_string();

    let process_path = fs::read_link(proc_path.join("exe"))
        .ok()?
        .to_string_lossy()
        .to_string();

    Some(ProcessInfo {
        pid,
        port,
        process_name,
        process_path,
    })
}
