use std::process::Command;

/// Set system proxy on macOS using networksetup
pub fn set_system_proxy(host: &str, port: u16) -> anyhow::Result<()> {
    // Get all network services
    let output = Command::new("networksetup")
        .args(["-listallnetworkservices"])
        .output()?;

    let services = String::from_utf8_lossy(&output.stdout);
    let services: Vec<&str> = services
        .lines()
        .skip(1)
        .filter(|s| !s.contains('*'))
        .collect();

    for service in services {
        let service = service.trim();
        if service.is_empty() {
            continue;
        }

        // Set SOCKS proxy
        let _ = Command::new("networksetup")
            .args([
                "-setsocksfirewallproxy",
                service,
                host,
                &port.to_string(),
            ])
            .output();

        // Set HTTP proxy for web traffic
        let _ = Command::new("networksetup")
            .args(["-setwebproxy", service, host, &port.to_string()])
            .output();

        let _ = Command::new("networksetup")
            .args([
                "-setsecurewebproxy",
                service,
                host,
                &port.to_string(),
            ])
            .output();

        // Enable the proxies
        let _ = Command::new("networksetup")
            .args(["-setsocksfirewallproxystate", service, "on"])
            .output();
        let _ = Command::new("networksetup")
            .args(["-setwebproxystate", service, "on"])
            .output();
        let _ = Command::new("networksetup")
            .args(["-setsecurewebproxystate", service, "on"])
            .output();
    }

    Ok(())
}

/// Disable system proxy on macOS
pub fn disable_system_proxy() -> anyhow::Result<()> {
    let output = Command::new("networksetup")
        .args(["-listallnetworkservices"])
        .output()?;

    let services = String::from_utf8_lossy(&output.stdout);
    let services: Vec<&str> = services
        .lines()
        .skip(1)
        .filter(|s| !s.contains('*'))
        .collect();

    for service in services {
        let service = service.trim();
        if service.is_empty() {
            continue;
        }

        let _ = Command::new("networksetup")
            .args(["-setsocksfirewallproxystate", service, "off"])
            .output();
        let _ = Command::new("networksetup")
            .args(["-setwebproxystate", service, "off"])
            .output();
        let _ = Command::new("networksetup")
            .args(["-setsecurewebproxystate", service, "off"])
            .output();
    }

    Ok(())
}
