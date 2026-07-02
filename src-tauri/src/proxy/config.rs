use crate::subscription::parser::ProxyNode;
use serde::{Deserialize, Serialize};

/// Generate a sing-box compatible configuration from a proxy node
/// sing-box config format: https://sing-box.sagernet.org/configuration/
pub fn generate_singbox_config(
    node: &ProxyNode,
    socks_port: u16,
    http_port: u16,
) -> serde_json::Value {
    serde_json::json!({
        "log": {
            "level": "info",
            "timestamp": true
        },
        "dns": {
            "servers": [
                {
                    "tag": "remote",
                    "address": "8.8.8.8",
                    "detour": "proxy"
                },
                {
                    "tag": "local",
                    "address": "223.5.5.5",
                    "detour": "direct"
                }
            ],
            "rules": [
                {
                    "rule_set": "geosite-cn",
                    "server": "local"
                }
            ]
        },
        "inbounds": [
            {
                "type": "socks",
                "tag": "socks-in",
                "listen": "127.0.0.1",
                "listen_port": socks_port,
                "sniff": true
            },
            {
                "type": "http",
                "tag": "http-in",
                "listen": "127.0.0.1",
                "listen_port": http_port
            }
        ],
        "outbounds": [
            {
                "type": node.protocol.as_str(),
                "tag": "proxy",
                "server": node.address.as_str(),
                "server_port": node.port,
                "uuid": node.uuid.as_str(),
                "flow": "",
                "transport": build_transport(node),
                "tls": build_tls(node),
                "multiplex": {
                    "enabled": true,
                    "protocol": "h2mux",
                    "max_connections": 4,
                    "min_streams": 2
                }
            },
            {
                "type": "direct",
                "tag": "direct"
            }
        ],
        "route": {
            "rules": [
                {
                    "rule_set": "geosite-cn",
                    "outbound": "direct"
                },
                {
                    "rule_set": "geoip-cn",
                    "outbound": "direct"
                }
            ],
            "auto_detect_interface": true,
            "final": "proxy"
        }
    })
}

fn build_transport(node: &ProxyNode) -> serde_json::Value {
    match node.transport.as_str() {
        "ws" => serde_json::json!({
            "type": "ws",
            "path": node.path,
            "headers": {
                "Host": node.host
            },
            "max_early_data": 2048,
            "early_data_header_name": "Sec-WebSocket-Protocol"
        }),
        "grpc" => serde_json::json!({
            "type": "grpc",
            "service_name": node.path.trim_start_matches('/')
        }),
        _ => serde_json::json!({
            "type": "tcp"
        }),
    }
}

fn build_tls(node: &ProxyNode) -> serde_json::Value {
    if node.security == "tls" {
        let mut tls = serde_json::json!({
            "enabled": true,
            "server_name": node.host,
            "utls": {
                "enabled": true,
                "fingerprint": node.fingerprint
            }
        });

        // Add alpn for proper TLS negotiation
        if let serde_json::Value::Object(ref mut map) = tls {
            map.insert(
                "alpn".to_string(),
                serde_json::json!(["http/1.1"]),
            );
        }

        tls
    } else {
        serde_json::json!({
            "enabled": false
        })
    }
}

/// Node configuration for the UI state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub subscription_url: String,
    pub auto_connect: bool,
    pub proxy_mode: String,
    pub socks_port: u16,
    pub http_port: u16,
    pub selected_node_id: Option<String>,
    pub whitelisted_apps: Vec<String>,
    pub custom_processes: Vec<String>,
    pub last_region: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            subscription_url: String::new(),
            auto_connect: false,
            proxy_mode: "system".to_string(),
            socks_port: 1080,
            http_port: 1081,
            selected_node_id: None,
            whitelisted_apps: vec![],
            custom_processes: vec![],
            last_region: "HK".to_string(),
        }
    }
}

pub fn config_path() -> std::path::PathBuf {
    let mut path = dirs_next().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("gamepp-config.json");
    path
}

fn dirs_next() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    { let home = std::env::var("HOME").ok()?; Some(std::path::PathBuf::from(home).join(".config")) }
    #[cfg(target_os = "windows")]
    { Some(std::path::PathBuf::from(std::env::var("APPDATA").ok()?)) }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { let home = std::env::var("HOME").ok()?; Some(std::path::PathBuf::from(home).join(".config")) }
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}
