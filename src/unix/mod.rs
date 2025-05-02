#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::fetch_ports;
pub use linux::get_processes_using_port;
pub use linux::kill_process;
