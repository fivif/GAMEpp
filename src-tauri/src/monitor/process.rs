use std::collections::HashSet;
use std::process::Command;

/// Get the PID of a process by its name
/// Matches case-insensitively and supports partial names
pub fn find_process_pid(name: &str) -> Option<u32> {
    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let name_lower = name.to_lowercase();

    for (pid, process) in system.processes() {
        let proc_name = process.name().to_string_lossy().to_lowercase();
        // Match if the process name contains the search term, or vice versa
        if proc_name.contains(&name_lower) || name_lower.contains(&proc_name) {
            return Some(pid.as_u32());
        }
    }
    None
}

/// Get all remote IPs a process is connected to
pub fn get_process_ips(pid: u32) -> HashSet<String> {
    let mut ips = HashSet::new();

    #[cfg(target_os = "macos")]
    {
        // Use lsof to get UDP/TCP connections
        if let Ok(output) = Command::new("lsof")
            .args(["-i", "-n", "-P", "-p", &pid.to_string()])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                // Parse IP from lsof output
                // Format: process pid user fd type device size/off node name
                // The last column looks like: TCP 192.168.1.1:12345->8.8.8.8:443 (ESTABLISHED)
                for part in line.split_whitespace() {
                    if let Some(arrow) = part.find("->") {
                        let remote = &part[arrow + 2..];
                        if let Some(colon) = remote.rfind(':') {
                            let ip = &remote[..colon];
                            if is_public_ip(ip) {
                                ips.insert(ip.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("netstat")
            .args(["-ano", "-p", "UDP"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let pid_str = pid.to_string();
            for line in stdout.lines() {
                if line.contains(&pid_str) {
                    // Parse remote address from netstat output
                    for part in line.split_whitespace() {
                        if part.contains(':') && !part.starts_with("127.") && !part.starts_with("0.") {
                            if let Some(colon) = part.rfind(':') {
                                let ip = &part[..colon];
                                if is_public_ip(ip) {
                                    ips.insert(ip.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ips
}

fn is_public_ip(ip: &str) -> bool {
    // Filter out localhost and private IPs
    !ip.starts_with("127.")
        && !ip.starts_with("0.")
        && !ip.starts_with("192.168.")
        && !ip.starts_with("10.")
        && !ip.starts_with("172.16.")
        && !ip.starts_with("172.17.")
        && !ip.starts_with("172.18.")
        && !ip.starts_with("172.19.")
        && !ip.starts_with("172.20.")
        && !ip.starts_with("172.21.")
        && !ip.starts_with("172.22.")
        && !ip.starts_with("172.23.")
        && !ip.starts_with("172.24.")
        && !ip.starts_with("172.25.")
        && !ip.starts_with("172.26.")
        && !ip.starts_with("172.27.")
        && !ip.starts_with("172.28.")
        && !ip.starts_with("172.29.")
        && !ip.starts_with("172.30.")
        && !ip.starts_with("172.31.")
        && ip != "*:*"
        && ip != "[::]"
}
