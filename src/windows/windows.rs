use windows::Win32::Foundation::NO_ERROR;
use windows::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6TABLE_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
    MIB_UDP6TABLE_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};
use windows::Win32::System::Threading::PROCESS_TERMINATE;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, TerminateProcess,
};

use windows::Win32::System::ProcessStatus::{K32GetModuleBaseNameW, K32GetModuleFileNameExW};

use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::os::windows::ffi::OsStringExt;

use crate::common::{KillProcessResponse, PortInfo, ProcessPortState};

const TCP_STATE_LISTEN: u32 = 2;

#[derive(Debug)]
enum Protocol {
    TcpIpv4,
    TcpIpv6,
    UdpIpv4,
    UdpIpv6,
}

fn get_buffer_size(protocol: &Protocol) -> Option<u32> {
    let mut buffer_size = 0u32;

    unsafe {
        let result = match protocol {
            Protocol::TcpIpv4 => {
                GetExtendedTcpTable(None, &mut buffer_size, false, 2, TCP_TABLE_OWNER_PID_ALL, 0)
            }
            Protocol::TcpIpv6 => GetExtendedTcpTable(
                None,
                &mut buffer_size,
                false,
                23,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ),
            Protocol::UdpIpv4 => {
                GetExtendedUdpTable(None, &mut buffer_size, false, 2, UDP_TABLE_OWNER_PID, 0)
            }
            Protocol::UdpIpv6 => {
                GetExtendedUdpTable(None, &mut buffer_size, false, 23, UDP_TABLE_OWNER_PID, 0)
            }
        };

        if result == 122 {
            Some(buffer_size)
        } else {
            println!("Unexpected result during first call: {}", result);
            None
        }
    }
}

fn fetch_table(protocol: &Protocol, buffer_size: u32) -> Option<Vec<u8>> {
    let mut buffer = vec![0u8; buffer_size as usize];

    unsafe {
        let result = match protocol {
            Protocol::TcpIpv4 => GetExtendedTcpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut (buffer_size as u32),
                false,
                2,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ),
            Protocol::TcpIpv6 => GetExtendedTcpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut (buffer_size as u32),
                false,
                23,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            ),
            Protocol::UdpIpv4 => GetExtendedUdpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut (buffer_size as u32),
                false,
                2,
                UDP_TABLE_OWNER_PID,
                0,
            ),
            Protocol::UdpIpv6 => GetExtendedUdpTable(
                Some(buffer.as_mut_ptr() as *mut _),
                &mut (buffer_size as u32),
                false,
                23,
                UDP_TABLE_OWNER_PID,
                0,
            ),
        };

        if result == NO_ERROR.0 {
            // println!("Successfully retrieved the table for protocol: {:?}", protocol);
            Some(buffer)
        } else {
            println!(
                "Failed to retrieve table for protocol: {:?}. Error code: {}",
                protocol, result
            );
            None
        }
    }
}

fn generate_unique_id(pid: u32, port: u16) -> String {
    let mut hasher = DefaultHasher::new();
    pid.hash(&mut hasher);
    port.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn parse_tcp_ipv4(buffer: &[u8]) -> Vec<PortInfo> {
    let mut results = Vec::new();

    unsafe {
        let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
        let rows = table.table.as_ptr();
        let count = table.dwNumEntries;

        for i in 0..count {
            let row = &*rows.add(i as usize);

            let port = u16::from_be(row.dwLocalPort as u16);

            let id = generate_unique_id(row.dwOwningPid, port);

            let (process_name, process_path) = match get_process_info(row.dwOwningPid) {
                Some((process_name, process_path)) => (process_name, process_path),
                None => (String::from("Unknown"), String::from("Unknown")),
            };
            let port_state = if row.dwState == TCP_STATE_LISTEN {
                ProcessPortState::Hosting
            } else {
                ProcessPortState::Using
            };

            let port_info = PortInfo {
                id,
                port,
                process_name,
                process_path,
                pid: row.dwOwningPid,
                port_state,
            };

            if !results
                .iter()
                .any(|entry: &PortInfo| entry.port == port && entry.pid == row.dwOwningPid)
            {
                results.push(port_info);
            }
        }
    }

    results
}

fn parse_tcp_ipv6(buffer: &[u8]) -> Vec<PortInfo> {
    let mut results = Vec::new();

    unsafe {
        let table = &*(buffer.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID);
        let rows = table.table.as_ptr();
        let count = table.dwNumEntries;

        for i in 0..count {
            let row = &*rows.add(i as usize);

            let port = u16::from_be(row.dwLocalPort as u16);

            let id = generate_unique_id(row.dwOwningPid, port);

            let (process_name, process_path) = match get_process_info(row.dwOwningPid) {
                Some((process_name, process_path)) => (process_name, process_path),
                None => (String::from("Unknown"), String::from("Unknown")),
            };
            let port_state = if row.dwState == TCP_STATE_LISTEN {
                ProcessPortState::Hosting
            } else {
                ProcessPortState::Using
            };

            let port_info = PortInfo {
                id,
                port,
                process_name,
                process_path,
                pid: row.dwOwningPid,
                port_state,
            };

            if !results
                .iter()
                .any(|entry: &PortInfo| entry.port == port && entry.pid == row.dwOwningPid)
            {
                results.push(port_info);
            }
        }
    }

    results
}

fn parse_udp_ipv4(buffer: &[u8]) -> Vec<PortInfo> {
    let mut results = Vec::new();

    unsafe {
        let table = &*(buffer.as_ptr() as *const MIB_UDPTABLE_OWNER_PID);
        let rows = table.table.as_ptr();
        let count = table.dwNumEntries;

        for i in 0..count {
            let row = &*rows.add(i as usize);

            let port = u16::from_be(row.dwLocalPort as u16);

            let id = generate_unique_id(row.dwOwningPid, port);

            let (process_name, process_path) = match get_process_info(row.dwOwningPid) {
                Some((process_name, process_path)) => (process_name, process_path),
                None => (String::from("Unknown"), String::from("Unknown")),
            };

            let port_info = PortInfo {
                id,
                port,
                process_name,
                process_path,
                pid: row.dwOwningPid,
                port_state: ProcessPortState::Using,
            };

            if !results
                .iter()
                .any(|entry: &PortInfo| entry.port == port && entry.pid == row.dwOwningPid)
            {
                results.push(port_info);
            }
        }
    }

    results
}

fn parse_udp_ipv6(buffer: &[u8]) -> Vec<PortInfo> {
    let mut results = Vec::new();

    unsafe {
        let table = &*(buffer.as_ptr() as *const MIB_UDP6TABLE_OWNER_PID);
        let rows = table.table.as_ptr();
        let count = table.dwNumEntries;

        for i in 0..count {
            let row = &*rows.add(i as usize);

            let port = u16::from_be(row.dwLocalPort as u16);

            let id = generate_unique_id(row.dwOwningPid, port);

            let (process_name, process_path) = match get_process_info(row.dwOwningPid) {
                Some((process_name, process_path)) => (process_name, process_path),
                None => (String::from("Unknown"), String::from("Unknown")),
            };

            let port_info = PortInfo {
                id,
                port,
                process_name,
                process_path,
                pid: row.dwOwningPid,
                port_state: ProcessPortState::Using,
            };

            if !results
                .iter()
                .any(|entry: &PortInfo| entry.port == port && entry.pid == row.dwOwningPid)
            {
                results.push(port_info);
            }
        }
    }

    results
}

pub fn fetch_ports() -> Result<Vec<crate::common::PortInfo>, String> {
    let protocols = [
        Protocol::TcpIpv4,
        Protocol::TcpIpv6,
        Protocol::UdpIpv4,
        Protocol::UdpIpv6,
    ];

    let mut all_connections = Vec::new();

    for protocol in protocols {
        match get_buffer_size(&protocol) {
            Some(buffer_size) => {
                if let Some(buffer) = fetch_table(&protocol, buffer_size) {
                    match protocol {
                        Protocol::TcpIpv4 => {
                            all_connections.extend(parse_tcp_ipv4(&buffer));
                        }
                        Protocol::TcpIpv6 => {
                            all_connections.extend(parse_tcp_ipv6(&buffer));
                        }
                        Protocol::UdpIpv4 => {
                            all_connections.extend(parse_udp_ipv4(&buffer));
                        }
                        Protocol::UdpIpv6 => {
                            all_connections.extend(parse_udp_ipv6(&buffer));
                        }
                    }
                } else {
                    return Err(format!(
                        "Failed to fetch table for protocol: {:?}",
                        protocol
                    ));
                }
            }
            None => {
                return Err(format!(
                    "Failed to get buffer size for protocol: {:?}",
                    protocol
                ));
            }
        }
    }

    Ok(all_connections)
}

pub fn kill_process(pid: u32) -> KillProcessResponse {
    unsafe {
        match OpenProcess(PROCESS_TERMINATE, false, pid) {
            Ok(process_handle) => {
                let terminate_result = TerminateProcess(process_handle, 1);
                let _ = CloseHandle(process_handle);

                match terminate_result {
                    Ok(()) => KillProcessResponse {
                        success: true,
                        message: format!("Successfully killed process with PID {}", pid),
                    },
                    Err(error) => {
                        let message = if error.code() == ERROR_ACCESS_DENIED.into() {
                            "Access denied".to_string()
                        } else {
                            format!("Error code: {:?}", error.code())
                        };
                        KillProcessResponse {
                            success: false,
                            message: format!(
                                "Failed to terminate process with PID {}: {}",
                                pid, message
                            ),
                        }
                    }
                }
            }
            Err(error) => KillProcessResponse {
                success: false,
                message: format!(
                    "Failed to open process with PID {}: {}",
                    pid,
                    error.message()
                ),
            },
        }
    }
}

pub fn get_process_info(pid: u32) -> Option<(String, String)> {
    unsafe {
        let process_handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut name_buffer = vec![0u16; 256];
        let mut path_buffer = vec![0u16; 1024];

        let name_len = K32GetModuleBaseNameW(Some(process_handle)?, None, &mut name_buffer);

        let process_name = if name_len > 0 {
            OsString::from_wide(&name_buffer[..name_len as usize])
                .to_string_lossy()
                .into_owned()
        } else {
            String::new()
        };

        let path_len = K32GetModuleFileNameExW(Some(process_handle), None, &mut path_buffer);

        let process_path = if path_len > 0 {
            OsString::from_wide(&path_buffer[..path_len as usize])
                .to_string_lossy()
                .into_owned()
        } else {
            String::new()
        };

        let _ = CloseHandle(process_handle);

        Some((process_name, process_path))
    }
}
