#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub enum ProcessPortState {
    Using,
    Hosting,
}

#[derive(serde::Serialize, Debug, Clone, Eq, Hash, PartialEq)]
pub struct PortInfo {
    pub id: String,
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub process_path: String,
    pub port_state: ProcessPortState,
}
impl PortInfo {
    pub fn ref_array(&self) -> Vec<String> {
        vec![
            self.port.to_string(),
            self.pid.to_string(),
            self.process_name.clone(),
            self.process_path.clone(),
            format!("{:?}", self.port_state),
        ]
    }
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
