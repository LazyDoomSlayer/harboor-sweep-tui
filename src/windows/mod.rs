#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use crate::windows::windows::fetch_ports;
pub use crate::windows::windows::kill_process;
