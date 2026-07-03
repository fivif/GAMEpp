use crate::monitor::{game_scanner, latency, process};
use crate::platform;
use crate::subscription::fetcher;
use crate::subscription::parser::{self, ProxyNode};
use crate::AppState;
use std::collections::HashSet;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn fetch_subscription(url: String) -> Result<Vec<ProxyNode>, String> {
    let content = fetcher::fetch_subscription(&url)
        .await
        .map_err(|e| format!("Fetch failed: {}", e))?;

    let nodes = parser::parse_node_list(&content);

    if nodes.is_empty() {
        // Return sample of the content for debugging
        let preview: String = content.chars().take(300).collect();
        return Err(format!(
            "No valid nodes found. Content length: {} bytes. Preview: {}",
            content.len(),
            preview
        ));
    }

    Ok(nodes)
}

#[tauri::command]
pub async fn parse_nodes(content: String) -> Result<Vec<ProxyNode>, String> {
    let nodes = parser::parse_node_list(&content);
    if nodes.is_empty() {
        return Err("No valid proxy nodes found".to_string());
    }
    Ok(nodes)
}

#[tauri::command]
pub async fn test_latency(nodes: Vec<ProxyNode>) -> Result<Vec<ProxyNode>, String> {
    // Test latency for all nodes
    let latencies = latency::test_nodes_latency(&nodes).await;

    let results: Vec<ProxyNode> = nodes
        .into_iter()
        .map(|mut node| {
            if let Some((_, Some(ms))) = latencies.iter().find(|(n, _)| n == &node.name) {
                node.latency_ms = Some(*ms);
            } else {
                node.latency_ms = None;
            }
            node
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn start_proxy(
    state: State<'_, Arc<AppState>>,
    node: ProxyNode,
) -> Result<String, String> {
    let socks_port = *state.proxy_port.lock();

    // Kill any existing sing-box process first
    {
        let mut proc = state.singbox_process.lock();
        if let Some(ref mut child) = *proc {
            let _ = child.kill();
        }
        *proc = None;
    }

    // Generate sing-box config and write to temp file
    let config = crate::proxy::config::generate_singbox_config(&node, socks_port, socks_port + 1);

    let config_path = std::env::temp_dir().join("gamepp-singbox-config.json");
    let config_str = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Config serialize: {}", e))?;
    std::fs::write(&config_path, &config_str)
        .map_err(|e| format!("Config write: {}", e))?;

    // Find sing-box binary - REQUIRED
    let sb_path = find_singbox()
        .ok_or_else(|| singbox_download_guide())?;

    // Start sing-box with stderr capture for debugging
    let stderr_file = std::env::temp_dir().join("gamepp-singbox-err.log");
    let stderr = std::fs::File::create(&stderr_file)
        .map_err(|e| format!("Cannot create log: {}", e))?;

    let mut cmd = std::process::Command::new(&sb_path);
    cmd.args(["run", "-c", config_path.to_str().unwrap()])
        .env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true")
        .env("ENABLE_DEPRECATED_MISSING_DOMAIN_RESOLVER", "true")
        .stdout(std::process::Stdio::null())
        .stderr(stderr);

    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let child = cmd.spawn()
        .map_err(|e| format!("Launch sing-box failed: {} (path: {})", e, sb_path))?;

    {
        let mut proc = state.singbox_process.lock();
        *proc = Some(child);
    }

    // Give sing-box time to start
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Verify the proxy is actually listening
    let verify = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], socks_port)),
        std::time::Duration::from_secs(2),
    );
    if verify.is_err() {
        // Kill the failed process
        let mut proc = state.singbox_process.lock();
        if let Some(ref mut child) = *proc {
            let _ = child.kill();
        }
        *proc = None;

        // Read stderr for diagnostics
        let err_log = std::fs::read_to_string(&stderr_file)
            .unwrap_or_default();
        return Err(format!("sing-box failed to start on port {}. Stderr: {}", socks_port, err_log));
    }

    // Update state
    *state.is_connected.lock() = true;
    *state.current_node.lock() = Some(node.name.clone());

    // Set system proxy
    platform::set_system_proxy("127.0.0.1", socks_port)
        .map_err(|e| format!("Proxy setting failed: {}", e))?;

    Ok(format!("Connected to {} via :{}", node.name, socks_port))
}

#[tauri::command]
pub async fn stop_proxy(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    // Kill sing-box process if running
    let mut proc = state.singbox_process.lock();
    if let Some(ref mut child) = *proc {
        if let Err(e) = child.kill() {
            tracing::warn!("Failed to kill sing-box: {}", e);
        }
    }
    *proc = None;

    // Disable system proxy
    if let Err(e) = platform::disable_system_proxy() {
        tracing::warn!("Failed to disable system proxy: {}", e);
    }

    // Update state
    *state.is_connected.lock() = false;
    *state.current_node.lock() = None;

    Ok("Disconnected".to_string())
}

#[tauri::command]
pub async fn get_connection_status(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "connected": *state.is_connected.lock(),
        "current_node": *state.current_node.lock(),
        "proxy_port": *state.proxy_port.lock(),
    }))
}

#[tauri::command]
pub async fn set_system_proxy(
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<(), String> {
    if enabled {
        let port = *state.proxy_port.lock();
        platform::set_system_proxy("127.0.0.1", port)
            .map_err(|e| format!("Failed to set proxy: {}", e))
    } else {
        platform::disable_system_proxy()
            .map_err(|e| format!("Failed to disable proxy: {}", e))
    }
}

#[tauri::command]
pub async fn get_running_apps() -> Result<Vec<String>, String> {
    use sysinfo::System;

    let mut system = System::new_all();
    system.refresh_all();

    let mut apps: Vec<String> = system
        .processes()
        .iter()
        .filter_map(|(_, process)| {
            let name = process.name().to_string_lossy().to_string();
            // Only show interesting processes (with .exe or .app extensions)
            if name.ends_with(".exe") || name.ends_with(".app") || name.contains('.') {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    apps.sort();
    apps.dedup();
    Ok(apps.into_iter().take(100).collect())
}

#[tauri::command]
pub async fn get_app_state_json(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "connected": *state.is_connected.lock(),
        "current_node": *state.current_node.lock(),
        "proxy_port": *state.proxy_port.lock(),
    }))
}

#[tauri::command]
pub async fn find_game_process(name: String) -> Result<Option<u32>, String> {
    Ok(process::find_process_pid(&name))
}

#[tauri::command]
pub async fn get_process_ips(pid: u32) -> Result<Vec<String>, String> {
    let ips: Vec<String> = process::get_process_ips(pid).into_iter().collect();
    Ok(ips)
}

#[tauri::command]
pub async fn scan_installed_games() -> Result<Vec<game_scanner::InstalledGame>, String> {
    Ok(game_scanner::scan_installed_games())
}

#[tauri::command]
pub async fn load_persistent_config() -> Result<crate::proxy::config::AppConfig, String> {
    Ok(crate::proxy::config::load_config())
}

#[tauri::command]
pub async fn save_persistent_config(config: crate::proxy::config::AppConfig) -> Result<(), String> {
    crate::proxy::config::save_config(&config).map_err(|e| e.to_string())
}

/// Find sing-box or auto-download it
fn find_singbox() -> Option<String> {
    // 1. Check PATH
    if std::process::Command::new("sing-box").arg("version").output().is_ok() {
        return Some("sing-box".to_string());
    }

    // 2. Check app data dir (where auto-download puts it)
    let app_dir = singbox_app_dir();
    #[cfg(target_os = "windows")]
    let name = "sing-box.exe";
    #[cfg(not(target_os = "windows"))]
    let name = "sing-box";

    let dest = app_dir.join(name);
    if dest.exists() { return Some(dest.to_string_lossy().to_string()); }

    // 3. Check next to exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let bundled = dir.join(name);
            if bundled.exists() { return Some(bundled.to_string_lossy().to_string()); }
        }
    }

    // 4. macOS Homebrew
    #[cfg(target_os = "macos")]
    for p in ["/opt/homebrew/bin/sing-box", "/usr/local/bin/sing-box"] {
        if std::path::Path::new(p).exists() { return Some(p.to_string()); }
    }

    // 5. Auto-download
    if let Ok(path) = auto_download_singbox(&dest) {
        return Some(path);
    }

    None
}

fn singbox_app_dir() -> std::path::PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from("."))
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join("Library/Application Support")).unwrap_or_else(|_| std::path::PathBuf::from("."))
    } else {
        std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")).unwrap_or_else(|_| std::path::PathBuf::from("."))
    };
    base.join("GAME++").join("bin")
}

fn auto_download_singbox(dest: &std::path::Path) -> Result<String, String> {
    let parent = dest.parent().ok_or("no parent dir")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {}", e))?;

    let url = if cfg!(target_os = "windows") {
        "https://github.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-windows-amd64.zip"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "https://github.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-darwin-arm64.tar.gz"
    } else {
        "https://github.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-darwin-amd64.tar.gz"
    };

    let tmp = std::env::temp_dir().join("gamepp-singbox-dl");
    std::fs::create_dir_all(&tmp).ok();

    let archive = if url.ends_with(".zip") {
        let p = tmp.join("sing-box.zip");
        download_file(url, &p)?;
        p
    } else {
        let p = tmp.join("sing-box.tar.gz");
        download_file(url, &p)?;
        p
    };

    // Extract
    if url.ends_with(".zip") {
        let file = std::fs::File::open(&archive).map_err(|e| format!("open zip: {}", e))?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("zip: {}", e))?;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| format!("zip entry: {}", e))?;
            let fname = std::path::Path::new(f.name()).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if fname == "sing-box.exe" || fname == "sing-box" {
                let mut out = std::fs::File::create(dest).map_err(|e| format!("create: {}", e))?;
                std::io::copy(&mut f, &mut out).map_err(|e| format!("copy: {}", e))?;
                break;
            }
        }
    } else {
        let f = std::fs::File::open(&archive).map_err(|e| format!("open tar: {}", e))?;
        let gz = flate2::read::GzDecoder::new(f);
        let mut tar = tar::Archive::new(gz);
        for entry in tar.entries().map_err(|e| format!("tar: {}", e))? {
            let mut entry = entry.map_err(|e| format!("entry: {}", e))?;
            let path = entry.path().map_err(|e| format!("path: {}", e))?;
            let fname = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if fname == "sing-box" || fname == "sing-box.exe" {
                entry.unpack(dest).map_err(|e| format!("unpack: {}", e))?;
                break;
            }
        }
    }

    let _ = std::fs::remove_dir_all(&tmp);
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(dest, PermissionsExt::from_mode(0o755)); }
    Ok(dest.to_string_lossy().to_string())
}

fn download_file(url: &str, dest: &std::path::Path) -> Result<(), String> {
    let resp = reqwest::blocking::get(url).map_err(|e| format!("download: {}", e))?;
    let bytes = resp.bytes().map_err(|e| format!("read: {}", e))?;
    std::fs::write(dest, &bytes).map_err(|e| format!("write: {}", e))?;
    Ok(())
}

fn singbox_download_guide() -> String {
    "Downloading sing-box... If this persists, check your network.".into()
}
