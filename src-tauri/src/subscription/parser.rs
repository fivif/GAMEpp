use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyNode {
    pub name: String,
    pub protocol: String,       // vless, trojan, ss, vmess, etc.
    pub address: String,
    pub port: u16,
    pub uuid: String,
    pub security: String,       // tls, none
    pub transport: String,      // ws, grpc, tcp, kcp, quic
    pub host: String,           // ws host / sni
    pub path: String,           // ws path
    pub encryption: String,     // encryption method
    pub fingerprint: String,    // tls fingerprint
    pub region: String,         // extracted from name tag
    pub latency_ms: Option<u64>,
    pub is_connected: bool,
    pub raw_url: String,
}

/// Parse a VLESS URL
/// Format: vless://uuid@address:port?params#name
fn parse_vless_url(raw: &str) -> Option<ProxyNode> {
    let without_prefix = raw.strip_prefix("vless://")?;
    let (credentials, rest) = without_prefix.split_once('@')?;
    let uuid = credentials.to_string();

    let (addr_part, after_addr) = rest.split_once(':')?;
    let address = addr_part.to_string();

    let (port_str, after_port) = after_addr.split_once('?')?;
    let port: u16 = port_str.parse().ok()?;

    // Parse params
    let (params_str, name) = after_port.split_once('#')?;
    let name = urlencoding_decode(name)?;

    let mut security = String::from("none");
    let mut transport = String::from("tcp");
    let mut host = String::new();
    let mut path = String::from("/");
    let mut encryption = String::from("none");
    let mut fingerprint = String::from("chrome");

    for param in params_str.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            match key {
                "security" => security = urlencoding_decode(value).unwrap_or_default(),
                "type" => transport = urlencoding_decode(value).unwrap_or_default(),
                "host" => host = urlencoding_decode(value).unwrap_or_default(),
                "path" => path = urlencoding_decode(value).unwrap_or_default(),
                "encryption" => encryption = urlencoding_decode(value).unwrap_or_default(),
                "fp" => fingerprint = urlencoding_decode(value).unwrap_or_default(),
                "sni" => {
                    // SNI is used for TLS, host header for WS
                    if host.is_empty() {
                        host = urlencoding_decode(value).unwrap_or_default();
                    }
                }
                _ => {}
            }
        }
    }

    // Extract region from name
    let region = extract_region(&name);

    Some(ProxyNode {
        name,
        protocol: "vless".to_string(),
        address,
        port,
        uuid,
        security,
        transport,
        host,
        path,
        encryption,
        fingerprint,
        region,
        latency_ms: None,
        is_connected: false,
        raw_url: raw.to_string(),
    })
}

/// Parse a Trojan URL
/// Format: trojan://password@address:port?params#name
fn parse_trojan_url(raw: &str) -> Option<ProxyNode> {
    let without_prefix = raw.strip_prefix("trojan://")?;
    let (password, rest) = without_prefix.split_once('@')?;

    let (addr_part, after_addr) = rest.split_once(':')?;
    let address = addr_part.to_string();

    let (port_str, after_port) = after_addr.split_once('?')?;
    let port: u16 = port_str.parse().ok()?;

    let (params_str, name) = after_port.split_once('#')?;
    let name = urlencoding_decode(name)?;

    let mut security = String::from("tls");
    let mut host = String::new();
    let mut path = String::from("/");
    let mut fingerprint = String::from("chrome");

    for param in params_str.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            match key {
                "security" => security = urlencoding_decode(value).unwrap_or_default(),
                "sni" => host = urlencoding_decode(value).unwrap_or_default(),
                "host" => host = urlencoding_decode(value).unwrap_or_default(),
                "path" => path = urlencoding_decode(value).unwrap_or_default(),
                "fp" => fingerprint = urlencoding_decode(value).unwrap_or_default(),
                "type" => {} // transport type
                _ => {}
            }
        }
    }

    let region = extract_region(&name);

    Some(ProxyNode {
        name,
        protocol: "trojan".to_string(),
        address,
        port,
        uuid: password.to_string(),
        security,
        transport: "tcp".to_string(),
        host,
        path,
        encryption: "none".to_string(),
        fingerprint,
        region,
        latency_ms: None,
        is_connected: false,
        raw_url: raw.to_string(),
    })
}

/// Parse a Shadowsocks URL
/// Format: ss://base64(method:password)@address:port#name
fn parse_ss_url(raw: &str) -> Option<ProxyNode> {
    let without_prefix = raw.strip_prefix("ss://")?;

    // SS URLs can have the method:password part base64 encoded
    let (encoded, rest) = without_prefix.split_once('@')?;
    let decoded = String::from_utf8(base64_decoded_bytes(encoded)?).ok()?;
    let (_method, _password) = decoded.split_once(':')?;

    let (addr_part, after_addr) = rest.split_once(':')?;
    let address = addr_part.to_string();

    let (port_str, after_port) = if let Some((p, a)) = after_addr.split_once('#') {
        (p, a.to_string())
    } else {
        (after_addr, String::new())
    };

    let port: u16 = port_str.parse().ok()?;
    let name = urlencoding_decode(&after_port).unwrap_or_else(|| after_port.clone());

    let region = extract_region(&name);

    Some(ProxyNode {
        name,
        protocol: "ss".to_string(),
        address,
        port,
        uuid: encoded.to_string(), // store the encoded method:password
        security: "none".to_string(),
        transport: "tcp".to_string(),
        host: String::new(),
        path: String::new(),
        encryption: "aes-256-gcm".to_string(),
        fingerprint: String::new(),
        region,
        latency_ms: None,
        is_connected: false,
        raw_url: raw.to_string(),
    })
}

/// Parse a single proxy URL, detecting the protocol
pub fn parse_node(line: &str) -> Option<ProxyNode> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if line.starts_with("vless://") {
        parse_vless_url(line)
    } else if line.starts_with("trojan://") {
        parse_trojan_url(line)
    } else if line.starts_with("ss://") {
        parse_ss_url(line)
    } else if line.starts_with("vmess://") {
        // VMess uses base64-encoded JSON, skip for now
        None
    } else {
        None
    }
}

/// Parse a list of nodes from subscription content (base64 decoded text)
/// Handles both plain URL lists and Clash YAML format
pub fn parse_node_list(content: &str) -> Vec<ProxyNode> {
    // Try URL line format first
    let url_nodes: Vec<ProxyNode> = content
        .lines()
        .filter_map(parse_node)
        .collect();

    if !url_nodes.is_empty() {
        return url_nodes;
    }

    // Try Clash YAML format
    parse_clash_yaml(content)
}

/// Parse Clash YAML subscription format
fn parse_clash_yaml(content: &str) -> Vec<ProxyNode> {
    let parsed: serde_yaml::Value = match serde_yaml::from_str(content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let proxies = match &parsed["proxies"] {
        serde_yaml::Value::Sequence(seq) => seq,
        _ => return vec![],
    };

    proxies
        .iter()
        .filter_map(|p| clash_proxy_to_node(p))
        .collect()
}

fn clash_proxy_to_node(proxy: &serde_yaml::Value) -> Option<ProxyNode> {
    let name = proxy["name"].as_str()?.to_string();
    let proto = proxy["type"].as_str()?.to_string();
    let address = proxy["server"].as_str()?.to_string();
    let port = proxy["port"].as_u64()? as u16;
    let uuid = proxy["uuid"].as_str().unwrap_or("").to_string();

    let security = if proxy["tls"].as_bool().unwrap_or(false) {
        "tls".to_string()
    } else {
        "none".to_string()
    };

    let transport = proxy["network"].as_str().unwrap_or("tcp").to_string();
    let host = proxy["servername"]
        .as_str()
        .or_else(|| proxy["ws-opts"]["headers"]["Host"].as_str())
        .unwrap_or("")
        .to_string();
    let path = proxy["ws-opts"]["path"]
        .as_str()
        .unwrap_or("/")
        .to_string();
    let fingerprint = proxy["client-fingerprint"].as_str().unwrap_or("chrome").to_string();
    let region = extract_region(&name);

    Some(ProxyNode {
        name,
        protocol: proto,
        address,
        port,
        uuid,
        security,
        transport,
        host,
        path,
        encryption: "none".to_string(),
        fingerprint,
        region,
        latency_ms: None,
        is_connected: false,
        raw_url: String::new(),
    })
}

/// Extract region code from node name
/// Names are like "SG|官方优选|100ms" or "HK 移动优选[75ms]"
fn extract_region(name: &str) -> String {
    // Try "XX|" pattern first
    for delimiter in &['|', ' '] {
        if let Some(first_part) = name.split(*delimiter).next() {
            let cleaned = first_part.trim();
            if cleaned.len() == 2
                && cleaned.chars().all(|c| c.is_ascii_uppercase())
                && cleaned != "CF"
            {
                return cleaned.to_string();
            }
            // Handle "CF电信优选" pattern
            if cleaned.len() >= 2 && &cleaned[..2] == "CF" {
                return "CF".to_string();
            }
        }
    }
    "Unknown".to_string()
}

/// Simple URL decode for percent-encoded strings
fn urlencoding_decode(input: &str) -> Option<String> {
    let mut bytes: Vec<u8> = Vec::with_capacity(input.len());

    for b in input.bytes() {
        match b {
            b'%' => {
                // We need to collect the hex bytes manually since we can't peek in a for loop
                // Delegate to a proper parser
            }
            b'+' => bytes.push(b' '),
            _ => bytes.push(b),
        }
    }

    // If no percent signs, return as-is
    if !input.contains('%') {
        return Some(input.replace('+', " "));
    }

    // Proper percent-decode: collect all bytes then convert to UTF-8 string
    let mut decoded: Vec<u8> = Vec::with_capacity(input.len());
    let mut i = 0;
    let input_bytes = input.as_bytes();
    while i < input_bytes.len() {
        if input_bytes[i] == b'%' && i + 2 < input_bytes.len() {
            let hi = hex_val(input_bytes[i + 1])?;
            let lo = hex_val(input_bytes[i + 2])?;
            decoded.push(hi * 16 + lo);
            i += 3;
        } else if input_bytes[i] == b'+' {
            decoded.push(b' ');
            i += 1;
        } else {
            decoded.push(input_bytes[i]);
            i += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Custom base64 decode that handles URL-safe base64 and missing padding
fn base64_decoded_bytes(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let mut input = input.to_string();

    // Replace URL-safe chars
    input = input.replace('-', "+").replace('_', "/");

    // Add padding if needed
    let padding = (4 - (input.len() % 4)) % 4;
    input.push_str(&"=".repeat(padding));

    base64::engine::general_purpose::STANDARD
        .decode(&input)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vless() {
        let url = "vless://7081af54-2a14-447d-8486-5173d0adff74@8.35.211.136:2083?security=tls&type=ws&host=pro.dl.214578.xyz&fp=chrome&sni=pro.dl.214578.xyz&path=%2F&encryption=none#CF%E7%94%B5%E4%BF%A1%E4%BC%98%E9%80%891";
        let node = parse_vless_url(url).unwrap();
        assert_eq!(node.protocol, "vless");
        assert_eq!(node.address, "8.35.211.136");
        assert_eq!(node.port, 2083);
        assert_eq!(node.transport, "ws");
        assert_eq!(node.security, "tls");
    }

    #[test]
    fn test_parse_region() {
        assert_eq!(extract_region("SG|官方优选|100ms"), "SG");
        assert_eq!(extract_region("HK 移动优选[75ms]"), "HK");
        assert_eq!(extract_region("JP|官方优选|86ms"), "JP");
        assert_eq!(extract_region("CF电信优选1"), "CF");
        assert_eq!(extract_region("DE|官方优选|232ms"), "DE");
    }

    #[test]
    fn test_parse_multi_node_sub() {
        // Simulate a subscription with multiple nodes across regions
        let content = r"vless://7081af54-2a14-447d-8486-5173d0adff74@8.35.211.136:2083?security=tls&type=ws&host=pro.dl.214578.xyz&fp=chrome&sni=pro.dl.214578.xyz&path=%2F&encryption=none#HK%7C%E5%AE%98%E6%96%B9%E4%BC%98%E9%80%89%7C60ms
vless://7081af54-2a14-447d-8486-5173d0adff74@108.162.198.57:2087?security=tls&type=ws&host=pro.dl.214578.xyz&fp=chrome&sni=pro.dl.214578.xyz&path=%2F&encryption=none#JP%7C%E5%AE%98%E6%96%B9%E4%BC%98%E9%80%89%7C110ms
vless://7081af54-2a14-447d-8486-5173d0adff74@103.31.4.187:443?security=tls&type=ws&host=pro.dl.214578.xyz&fp=chrome&sni=pro.dl.214578.xyz&path=%2F&encryption=none#US%7C%E5%AE%98%E6%96%B9%E4%BC%98%E9%80%89%7C243ms
trojan://testpass@1.2.3.4:443?security=tls&sni=test.com#DE%7Ctest
";
        let nodes = parse_node_list(content);
        println!("Parsed {} nodes from multi-node subscription", nodes.len());
        for n in &nodes {
            println!("  {} | {}:{} | proto={}", n.name, n.address, n.port, n.protocol);
        }
        assert!(nodes.len() >= 3, "Should parse at least 3 VLESS nodes, got {}", nodes.len());
    }
}
