#[derive(serde::Serialize, Debug)]
pub enum ProcessPortState {
    Hosting,
    Using,
}
#[derive(serde::Serialize, Debug)]
pub struct PortInfo {
    pub id: String,
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub process_path: String,
    pub port_state: ProcessPortState,
}

#[derive(serde::Serialize, Debug)]
pub struct KillProcessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(serde::Serialize, Debug)]
pub struct ProcessInfoResponse {
    pub port_state: ProcessPortState,
    pub data: Option<ProcessInfo>,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub port: u16,
    pub process_name: String,
    pub process_path: String,
}
