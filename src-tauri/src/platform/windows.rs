/// Set system proxy on Windows via registry
pub fn set_system_proxy(host: &str, port: u16) -> anyhow::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        KEY_WRITE,
    )?;

    // Enable proxy
    internet_settings.set_value("ProxyEnable", &1u32)?;

    // Set proxy server (SOCKS5 + HTTP)
    let proxy_addr = format!("http={}:{};https={}:{};socks={}:{}",
        host, port, host, port, host, port);
    internet_settings.set_value("ProxyServer", &proxy_addr)?;

    // Set proxy override (bypass for local)
    internet_settings.set_value("ProxyOverride", &"<local>")?;

    Ok(())
}

/// Disable system proxy on Windows
pub fn disable_system_proxy() -> anyhow::Result<()> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        KEY_WRITE,
    )?;

    internet_settings.set_value("ProxyEnable", &0u32)?;

    Ok(())
}
