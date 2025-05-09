pub mod common;
pub use common::{
    KillProcessResponse, PortInfo, ProcessInfo, ProcessInfoResponse, ProcessPortState,
};

#[cfg(target_family = "unix")]
mod unix;

#[cfg(target_family = "unix")]
pub(crate) mod os {
    pub use super::unix::{fetch_ports, kill_process};
}

#[cfg(target_family = "windows")]
mod windows;

#[cfg(target_family = "windows")]
pub(crate) mod os {
    pub use super::windows::{fetch_ports, kill_process};
}
